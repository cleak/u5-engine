//! Intro-menu key dispatch per `intro.md` §6.

use std::io;
use std::path::Path;

use crate::{input_case_fold, read_optional_disk_file};

/// `intro.md §3` title-screen surface dimensions. The title flow
/// places its bitmap slots inside a fixed 320-by-200 pixel coordinate
/// system with the origin at the upper-left corner. Promote the
/// width and height so the title-tick rectangle and the bitmap
/// placements share one named source of truth instead of comparing
/// against bare `320` / `200` literals.
pub const TITLE_SURFACE_WIDTH: u16 = 320;
pub const TITLE_SURFACE_HEIGHT: u16 = 200;

/// `intro.md §3`: title-screen 320x200 pixel placement for one
/// bitmap slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TitleBitPlacement {
    pub asset: TitleBitAsset,
    pub slot: u8,
    pub top_left_x: u16,
    pub top_left_y: u16,
    pub width: u16,
    pub height: u16,
}

/// `intro.md §3`: which compressed-bitmap resource a placement
/// references.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleBitAsset {
    Title,
    British,
}

/// `intro.md §3` hidden initial title source placements. `TITLE.BIT`
/// slots 0..=6 are stacked into this off-screen source surface before
/// the title animation player reveals selected rows.
pub const TITLE_BIT_INITIAL_SOURCE_PLACEMENTS: [TitleBitPlacement; 7] = [
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 0,
        top_left_x: 148,
        top_left_y: 0,
        width: 24,
        height: 3,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 1,
        top_left_x: 140,
        top_left_y: 3,
        width: 40,
        height: 7,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 2,
        top_left_x: 124,
        top_left_y: 10,
        width: 72,
        height: 11,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 3,
        top_left_x: 104,
        top_left_y: 21,
        width: 112,
        height: 20,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 4,
        top_left_x: 84,
        top_left_y: 41,
        width: 152,
        height: 32,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 5,
        top_left_x: 52,
        top_left_y: 73,
        width: 216,
        height: 45,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 6,
        top_left_x: 20,
        top_left_y: 118,
        width: 280,
        height: 61,
    },
];

/// `intro.md §3` visible initial title mark placements. `TITLE.BIT`
/// slots 0..=6 are presented one at a time from the hidden source
/// surface, replacing the previous visible flourish frame rather than
/// accumulating.
pub const TITLE_BIT_INITIAL_PLACEMENTS: [TitleBitPlacement; 7] = [
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 0,
        top_left_x: 148,
        top_left_y: 75,
        width: 24,
        height: 3,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 1,
        top_left_x: 140,
        top_left_y: 72,
        width: 40,
        height: 7,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 2,
        top_left_x: 124,
        top_left_y: 71,
        width: 72,
        height: 11,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 3,
        top_left_x: 104,
        top_left_y: 66,
        width: 112,
        height: 20,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 4,
        top_left_x: 84,
        top_left_y: 60,
        width: 152,
        height: 32,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 5,
        top_left_x: 52,
        top_left_y: 53,
        width: 216,
        height: 45,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 6,
        top_left_x: 20,
        top_left_y: 46,
        width: 280,
        height: 61,
    },
];

/// `intro.md §3` four `BRITISH.PTH` pen origins, in the order the
/// path walker is called.
pub const BRITISH_PTH_PEN_ORIGINS: [(u8, u8); 4] = [(68, 44), (94, 64), (78, 143), (105, 167)];

/// `intro.md §6.1` lower intro menu/text-window frame: anchor cell
/// (column, row) where the 40-wide × 10-tall rectangle begins.
pub const INTRO_MENU_FRAME_ANCHOR_COLUMN: u8 = 0;
pub const INTRO_MENU_FRAME_ANCHOR_ROW: u8 = 15;
pub const INTRO_MENU_FRAME_WIDTH_CELLS: u8 = 40;
pub const INTRO_MENU_FRAME_HEIGHT_CELLS: u8 = 10;
/// `intro.md §6.1` horizontal-rule pixel coordinates drawn through
/// the display-driver line primitive immediately under the top-edge
/// row.
pub const INTRO_MENU_FRAME_RULE_Y: u16 = 127;
pub const INTRO_MENU_FRAME_RULE_X0: u16 = 7;
pub const INTRO_MENU_FRAME_RULE_X1: u16 = 312;
/// `intro.md §6.1`: the five-glyph reserved corner/edge set the
/// fixed-cell font carries for boxed intro text. Per the published
/// IBM.CH glyph shapes at codes 0x7B-0x7F, the assignments are:
/// 0x7E = top-left (top + left solid, curve carved at bottom-right);
/// 0x7D = top-right (top + right solid, curve at bottom-left);
/// 0x7C = bottom-left (bottom + left solid, curve at top-right);
/// 0x7B = bottom-right (bottom + right solid, curve at top-left);
/// 0x7F = shared edge (solid 8×8 block, used for both horizontal
/// and vertical edges).
pub const INTRO_MENU_FRAME_GLYPH_TOP_LEFT: u8 = 0x7E;
pub const INTRO_MENU_FRAME_GLYPH_TOP_RIGHT: u8 = 0x7D;
pub const INTRO_MENU_FRAME_GLYPH_BOTTOM_LEFT: u8 = 0x7C;
pub const INTRO_MENU_FRAME_GLYPH_BOTTOM_RIGHT: u8 = 0x7B;
pub const INTRO_MENU_FRAME_GLYPH_EDGE: u8 = 0x7F;

/// `intro.md §3` remaining title-sequence bitmap placements drawn
/// after the seven-slot initial title mark. Order is `TITLE.BIT` 7,
/// `TITLE.BIT` 8, `BRITISH.BIT` 0, `TITLE.BIT` 9. The lower-band
/// clear at [`TITLE_LOWER_BAND_CLEAR_Y`] runs before slot 7.
pub const TITLE_BIT_REMAINING_PLACEMENTS: [TitleBitPlacement; 4] = [
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 7,
        top_left_x: 108,
        top_left_y: 140,
        width: 104,
        height: 33,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 8,
        top_left_x: 152,
        top_left_y: 0,
        width: 16,
        height: 15,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::British,
        slot: 0,
        top_left_x: 24,
        top_left_y: 66,
        width: 272,
        height: 62,
    },
    TitleBitPlacement {
        asset: TitleBitAsset::Title,
        slot: 9,
        top_left_x: 104,
        top_left_y: 160,
        width: 112,
        height: 33,
    },
];

/// `intro.md §3` lower-screen Y where the title flow clears the
/// lower band before drawing `TITLE.BIT` slot 7.
pub const TITLE_LOWER_BAND_CLEAR_Y: u16 = 140;

/// `intro.md §5` title-tick frame rectangle. The intro menu's idle
/// title-tick path draws one driver-local frame strip over the
/// title screen at this fixed pixel rectangle, then advances the
/// driver-local frame index modulo four. The replacement frames
/// belong to a cleanroom renderer; the cadence and destination
/// rectangle are part of the public contract.
pub const TITLE_TICK_FRAME_X: u16 = 0;
pub const TITLE_TICK_FRAME_Y: u16 = 65;
pub const TITLE_TICK_FRAME_WIDTH: u16 = TITLE_SURFACE_WIDTH;
pub const TITLE_TICK_FRAME_HEIGHT: u16 = 49;
pub const TITLE_TICK_FRAME_COUNT: u8 = 4;
pub const TITLE_TICK_FRAME_PIXELS: usize =
    TITLE_TICK_FRAME_WIDTH as usize * TITLE_TICK_FRAME_HEIGHT as usize;
pub const TITLE_TICK_FRAME_SET_BYTES: usize =
    TITLE_TICK_FRAME_PIXELS * TITLE_TICK_FRAME_COUNT as usize;

/// `intro.md §5`: advance the title-tick frame index modulo four.
pub const fn title_tick_next_frame(current_frame: u8) -> u8 {
    (current_frame + 1) % TITLE_TICK_FRAME_COUNT
}

/// `cleak/u5-spec#52` published title-tick palette cycle. Each frame
/// pairs an EGA bright index (drawn on the upper half of the flame
/// silhouette) with an EGA dim index (drawn on the lower half). The
/// four-frame loop drives the "wavering flame stripe" perceived
/// effect over the title-tick rectangle without changing the
/// underlying silhouette.
pub const TITLE_TICK_PALETTE_CYCLE: [(u8, u8); TITLE_TICK_FRAME_COUNT as usize] = [
    (0x0E, 0x06), // frame 0: light yellow over brown
    (0x0C, 0x04), // frame 1: light red over red
    (0x0E, 0x04), // frame 2: light yellow over red
    (0x0C, 0x06), // frame 3: light red over brown
];

/// `cleak/u5-spec#52`: returns `(bright_index, dim_index)` EGA
/// palette indices for the given mod-four title-tick frame.
pub const fn title_tick_palette_indices(frame: u8) -> (u8, u8) {
    let frame = (frame % TITLE_TICK_FRAME_COUNT) as usize;
    TITLE_TICK_PALETTE_CYCLE[frame]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleTickFrameSet {
    pixels: Vec<u8>,
}

impl TitleTickFrameSet {
    pub fn from_palette_indices(pixels: Vec<u8>, source: &str) -> io::Result<Self> {
        if pixels.len() != TITLE_TICK_FRAME_SET_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{source}: authored title-tick frame set must be exactly {TITLE_TICK_FRAME_SET_BYTES} bytes ({} frames of {}x{}), found {}",
                    TITLE_TICK_FRAME_COUNT,
                    TITLE_TICK_FRAME_WIDTH,
                    TITLE_TICK_FRAME_HEIGHT,
                    pixels.len()
                ),
            ));
        }
        if let Some((index, value)) = pixels
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| *value > 0x0f)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{source}: authored title-tick frame byte {index} has palette index 0x{value:02x}; expected EGA index 0x00..0x0f"
                ),
            ));
        }
        Ok(Self { pixels })
    }

    pub fn frame_pixels(&self, frame: u8) -> &[u8] {
        let frame = usize::from(frame % TITLE_TICK_FRAME_COUNT);
        let start = frame * TITLE_TICK_FRAME_PIXELS;
        &self.pixels[start..start + TITLE_TICK_FRAME_PIXELS]
    }
}

pub fn authored_title_tick_frames() -> &'static TitleTickFrameSet {
    panic!(
        "title-tick animation requires published authored frame pixels; generated clean-room frames are a forbidden fallback; see cleak/u5-spec#65"
    )
}

/// `systems/display-driver.md §5` source layout for the EGA driver's
/// title-tick frame strip. The spec publishes the band geometry — four
/// 320-pixel-wide bands at a 50-row source stride, with each tick
/// copying the upper 49 rows — but the exact byte offset within
/// `EGA.DRV` is a driver-revision-dependent locator that has not been
/// published clean-room-safely yet (tracked upstream as a follow-up
/// to `cleak/u5-spec#65`).
///
/// Callers parameterise the parser with this descriptor so the engine
/// can adopt the published locator immediately when the spec adds
/// one, without restructuring the extraction code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EgaTitleTickLayout {
    /// Byte offset within `EGA.DRV` where band 0 begins.
    pub start_offset: usize,
    /// Bytes per source row across all four EGA planes. The standard
    /// 320-pixel 4-plane packed layout is 40 bytes per plane × 4
    /// planes = 160 bytes per row.
    pub bytes_per_row: usize,
    /// Per-plane byte stride within a row (40 bytes for a
    /// 320-pixel-wide row at 1 bit per pixel per plane).
    pub plane_stride_bytes: usize,
    /// Total source rows per band (50, with the upper 49 copied to
    /// the destination rectangle and the bottom row discarded per
    /// the published §5 contract).
    pub source_rows_per_band: usize,
}

impl EgaTitleTickLayout {
    /// Standard 320-pixel-wide 4-plane EGA packed-row layout: 40
    /// bytes per plane × 4 planes = 160 bytes per row, with planes
    /// stored sequentially within each row (P0 P1 P2 P3). The
    /// `start_offset` is caller-supplied since the published
    /// `EGA.DRV` locator is still pending.
    pub const fn standard_4_plane(start_offset: usize) -> Self {
        Self {
            start_offset,
            bytes_per_row: 160,
            plane_stride_bytes: 40,
            source_rows_per_band: 50,
        }
    }

    pub const fn band_stride_bytes(&self) -> usize {
        self.bytes_per_row * self.source_rows_per_band
    }
}

/// `systems/display-driver.md §5` clean-room-safe extractor for the
/// title-tick four-frame strip from a runtime `EGA.DRV` image. The
/// caller supplies the layout descriptor; this function performs no
/// disassembly or address-derived reading and operates only on the
/// caller-provided byte range. Plane bytes are unpacked into per-pixel
/// EGA palette indices and the upper [`TITLE_TICK_FRAME_HEIGHT`] rows
/// of each band are emitted in band-major order to the returned
/// `TitleTickFrameSet`.
pub fn parse_ega_drv_title_tick_frames(
    bytes: &[u8],
    layout: EgaTitleTickLayout,
) -> io::Result<TitleTickFrameSet> {
    let total_bands = TITLE_TICK_FRAME_COUNT as usize;
    let band_stride = layout.band_stride_bytes();
    let required = layout
        .start_offset
        .checked_add(band_stride.checked_mul(total_bands).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "EGA.DRV title-tick layout overflows: 4 * band_stride exceeds usize",
            )
        })?)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "EGA.DRV title-tick layout overflows: start_offset + 4 * band_stride exceeds usize",
            )
        })?;
    if bytes.len() < required {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!(
                "EGA.DRV title-tick parser needs at least {required} bytes from offset {} (have {})",
                layout.start_offset,
                bytes.len()
            ),
        ));
    }
    if layout.plane_stride_bytes * 4 != layout.bytes_per_row {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "EGA.DRV title-tick layout invalid: plane_stride_bytes * 4 ({}) must equal bytes_per_row ({})",
                layout.plane_stride_bytes * 4,
                layout.bytes_per_row,
            ),
        ));
    }
    let width = TITLE_TICK_FRAME_WIDTH as usize;
    let height = TITLE_TICK_FRAME_HEIGHT as usize;

    let mut pixels = Vec::with_capacity(TITLE_TICK_FRAME_SET_BYTES);
    for band in 0..total_bands {
        let band_offset = layout.start_offset + band * band_stride;
        // §5: the destination receives the upper `height` rows of
        // the `source_rows_per_band`-row source band.
        for row in 0..height {
            let row_offset = band_offset + row * layout.bytes_per_row;
            for x in 0..width {
                let byte_index = x / 8;
                let bit_index = 7 - (x % 8);
                let plane_base = row_offset + byte_index;
                let p0 = (bytes[plane_base] >> bit_index) & 1;
                let p1 = (bytes[plane_base + layout.plane_stride_bytes] >> bit_index) & 1;
                let p2 = (bytes[plane_base + layout.plane_stride_bytes * 2] >> bit_index) & 1;
                let p3 = (bytes[plane_base + layout.plane_stride_bytes * 3] >> bit_index) & 1;
                pixels.push(p0 | (p1 << 1) | (p2 << 2) | (p3 << 3));
            }
        }
    }
    TitleTickFrameSet::from_palette_indices(pixels, "EGA.DRV title-tick strip")
}

/// `systems/display-driver.md §5` development placeholder. Returns a
/// four-frame strip of all-zero (black) pixels that satisfies the
/// public destination contract (320×49 per frame, four frames,
/// in-range EGA palette indices) without claiming any visual fidelity
/// to the original `EGA.DRV` frames. Use this only when the published
/// `EGA.DRV` locator is not yet wired up; the visible result is an
/// honest black band that surfaces the missing asset rather than a
/// synthesised animation that hides it.
pub fn placeholder_title_tick_frames() -> TitleTickFrameSet {
    TitleTickFrameSet::from_palette_indices(
        vec![0u8; TITLE_TICK_FRAME_SET_BYTES],
        "placeholder title-tick frames",
    )
    .expect("placeholder title-tick frame set is well-formed by construction")
}

/// `systems/display-driver.md §5` + `cleak/u5-spec#52` clean-room
/// authored title-tick strip. Produces four 320×49 frames whose
/// silhouette is a procedurally-generated wavering flame band
/// (deterministic, independently authored) and whose pixel palette
/// follows the published palette cycle exactly: bright index on the
/// upper half of the silhouette, dim index on the lower half, black
/// elsewhere. This is NOT pixel-identical to the historical
/// `EGA.DRV` frames — the spec explicitly says "exact reuse of the
/// historical driver-resident pixels is a driver-binary parity
/// issue, not an asset-format requirement" — but it satisfies every
/// public contract the spec ratifies: destination rectangle,
/// four-frame cadence, palette cycle, opaque overwrite, and visible
/// wavering effect.
///
/// The four silhouettes differ by phase so the eye sees motion in
/// the band, matching the §5 "wavering flame stripe perceived
/// effect" description.
pub fn clean_room_authored_title_tick_frames() -> TitleTickFrameSet {
    let width = TITLE_TICK_FRAME_WIDTH as usize;
    let height = TITLE_TICK_FRAME_HEIGHT as usize;
    let mut pixels = Vec::with_capacity(TITLE_TICK_FRAME_SET_BYTES);
    for frame in 0..TITLE_TICK_FRAME_COUNT as usize {
        let (bright, dim) = TITLE_TICK_PALETTE_CYCLE[frame];
        // Per-column flame height profile. Use a simple deterministic
        // multi-frequency sum to get a wavering crest that varies by
        // frame phase. Phase shifts in 1/4-cycle steps across the
        // four-frame loop so the silhouette appears to move with the
        // palette swap.
        let phase = frame as f32 * 0.5;
        let upper_split = height / 2;
        for row in 0..height {
            for col in 0..width {
                // Flame "tip" height varies sinusoidally per column.
                let crest_factor = 0.55
                    + 0.25 * ((col as f32) * 0.045 + phase).sin()
                    + 0.20 * ((col as f32) * 0.013 - phase * 1.7).sin()
                    + 0.10 * ((col as f32) * 0.085 + phase * 2.3).cos();
                let crest_row = (height as f32 * crest_factor.clamp(0.15, 0.95)) as usize;
                let lit = row >= crest_row;
                let pixel = if !lit {
                    0
                } else if row < upper_split {
                    bright & 0x0f
                } else {
                    dim & 0x0f
                };
                pixels.push(pixel);
            }
        }
    }
    TitleTickFrameSet::from_palette_indices(pixels, "clean-room authored title-tick frames")
        .expect("clean-room authored title-tick frame set is well-formed by construction")
}

/// `intro.md §12`: Return-to-View loads `MISCMAPS.DAT`. The first
/// four records are shown as 4-by-19 map strips, followed by a
/// 655-byte command stream driving preview actors and animation beats.
pub const MISCMAPS_DAT_FILE: &str = "MISCMAPS.DAT";
pub const RTV_STRIP_COUNT: usize = 4;
pub const RTV_STRIP_ROWS: usize = 19;
pub const RTV_STRIP_COLUMNS: usize = 4;
pub const RTV_COMMAND_STREAM_BYTES: usize = 655;

/// `formats/location-dat.md §11` MISCMAPS section offsets. The file
/// concatenates three sections back-to-back: four cutscene maps
/// (704 bytes), four Return-to-View strips (512 bytes), and the
/// 655-byte Return-to-View command stream. Promote the offsets and
/// per-section lengths so loader code does not bake them as bare
/// literals.
pub const MISCMAPS_CUTSCENE_SECTION_OFFSET: usize = 0;
/// `formats/location-dat.md §11` byte length of the cutscene-map
/// section: four maps of `16 × 11 = 176` bytes each (eleven 16-byte
/// rows with five trailing pad bytes), totalling 704 bytes.
pub const MISCMAPS_CUTSCENE_SECTION_BYTES: usize = 704;
/// `formats/location-dat.md §11` per-cutscene-map row stride. Each
/// cutscene map is authored as eleven 16-byte rows; the first 11
/// bytes of each row carry tile data and the trailing five bytes
/// are zero-padded.
pub const MISCMAPS_CUTSCENE_ROW_STRIDE: usize = 16;
pub const MISCMAPS_CUTSCENE_ROWS: usize = 11;
pub const MISCMAPS_CUTSCENE_VISIBLE_COLUMNS: usize = 11;
pub const MISCMAPS_CUTSCENE_RECORD_COUNT: usize = 4;
pub const MISCMAPS_CUTSCENE_RECORD_BYTES: usize =
    MISCMAPS_CUTSCENE_ROW_STRIDE * MISCMAPS_CUTSCENE_ROWS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiscmapsCutsceneMap {
    pub record_index: usize,
    pub tiles: Vec<u8>,
}

impl MiscmapsCutsceneMap {
    pub fn tile(&self, x: usize, y: usize) -> Option<u8> {
        (x < MISCMAPS_CUTSCENE_VISIBLE_COLUMNS && y < MISCMAPS_CUTSCENE_ROWS)
            .then(|| self.tiles[y * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS + x])
    }
}

pub fn load_miscmaps_cutscene_map(
    game_dir: &Path,
    record_index: usize,
) -> io::Result<Option<MiscmapsCutsceneMap>> {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    parse_miscmaps_cutscene_map_file(&bytes, record_index).map(Some)
}

pub fn require_miscmaps_cutscene_map(
    game_dir: &Path,
    record_index: usize,
) -> io::Result<MiscmapsCutsceneMap> {
    load_miscmaps_cutscene_map(game_dir, record_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("{MISCMAPS_DAT_FILE}: required cutscene map resource is missing"),
        )
    })
}

pub fn parse_miscmaps_cutscene_map_file(
    bytes: &[u8],
    record_index: usize,
) -> io::Result<MiscmapsCutsceneMap> {
    if record_index >= MISCMAPS_CUTSCENE_RECORD_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "MISCMAPS cutscene record must be 0..{}, got {record_index}",
                MISCMAPS_CUTSCENE_RECORD_COUNT - 1
            ),
        ));
    }
    let start = MISCMAPS_CUTSCENE_SECTION_OFFSET + record_index * MISCMAPS_CUTSCENE_RECORD_BYTES;
    let end = start + MISCMAPS_CUTSCENE_RECORD_BYTES;
    if bytes.len() < end {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!(
                "{MISCMAPS_DAT_FILE}: expected at least {end} bytes for cutscene record {record_index}, found {}",
                bytes.len()
            ),
        ));
    }

    let mut tiles = Vec::with_capacity(MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS);
    for row in 0..MISCMAPS_CUTSCENE_ROWS {
        let row_start = start + row * MISCMAPS_CUTSCENE_ROW_STRIDE;
        tiles.extend_from_slice(&bytes[row_start..row_start + MISCMAPS_CUTSCENE_VISIBLE_COLUMNS]);
    }
    Ok(MiscmapsCutsceneMap {
        record_index,
        tiles,
    })
}

/// `formats/location-dat.md §11` Return-to-View map strip section
/// offset (immediately after the cutscene section).
pub const MISCMAPS_RTV_STRIP_SECTION_OFFSET: usize = MISCMAPS_CUTSCENE_SECTION_BYTES;
/// `formats/location-dat.md §11` Return-to-View map strip section
/// byte length: four strips stored as four 32-byte source columns each,
/// totalling `4 * 32 * 4 = 512` bytes. The Return-to-View loader
/// loads the visible 4x19 source cells into the public 4x19
/// preview geometry.
pub const MISCMAPS_RTV_STRIP_SECTION_BYTES: usize = 512;
/// `formats/location-dat.md §11` per-strip column stride. Each strip
/// is authored as four 32-byte source columns; the first 19 bytes per
/// column carry tile data and the trailing 13 bytes are zero-padded.
pub const MISCMAPS_RTV_STRIP_ROW_STRIDE: usize = 32;

/// `formats/location-dat.md §11` Return-to-View command stream
/// section offset (immediately after the strip section).
pub const MISCMAPS_RTV_COMMAND_SECTION_OFFSET: usize =
    MISCMAPS_RTV_STRIP_SECTION_OFFSET + MISCMAPS_RTV_STRIP_SECTION_BYTES;
/// `intro.md §12`: Return-to-View command stream is interpreted as a
/// 16-command preview bytecode, not the gameplay TLK runner.
pub const RTV_COMMAND_COUNT: usize = 16;

/// `intro.md §11` describes the acknowledgements path as an artwork
/// screen: load a graphics resource from the end-screen asset family,
/// draw the credits artwork at a fixed position, reveal it with a
/// bottom-up slab wipe at a fixed pixel stride, wait for a key, wipe
/// it away top-to-bottom, then reload `STARTSC` and redraw the menu.
/// What is missing is the binding and the numbers, not prose: which
/// file and directory slot carries the credits artwork, its fixed
/// top-left origin, the slab height / pixel stride and per-slab tick
/// cadence for the two wipes, and whether any text is drawn over the
/// artwork at all (`§15` still defers text and pagination to a
/// source-free transcription, which `§11` may have made moot).
pub fn require_acknowledgements_contract() -> ! {
    panic!(
        "intro acknowledgements require the published end-screen file/slot binding for the credits artwork, its fixed draw origin, and the slab stride and cadence of the bottom-up entry and top-down exit wipes (intro.md §11); guessing the resource slot or geometry, or substituting clean-room-authored placeholder credits, is a forbidden fallback; see cleak/u5-spec#72 and cleak/u5-spec#82"
    )
}

/// `intro.md §6`: the six accepted intro-menu actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroMenuAction {
    /// `J` — load the active save and return to the main loop on success.
    JourneyOnward,
    /// `C` — enter character creation through the proportional-font /
    /// chargen flow.
    CreateNewCharacter,
    /// `T` — enter the Ultima IV transfer/roster path.
    TransferFromUltimaIv,
    /// `U` — play the story slide sequence and return to the menu.
    UltimaVIntroduction,
    /// `A` — show acknowledgements/credits and return to the menu.
    Acknowledgements,
    /// `R` — run the non-interactive Return-to-View preview and return.
    ReturnToView,
    /// Repeat the most-recent cached selection (Enter when a cache is
    /// present); caller maintains the cache and resolves it back to one
    /// of the six actions above. This variant signals the intent rather
    /// than the resolved action.
    RepeatCachedSelection,
}

/// `intro.md §6`: classify a raw key byte into an intro-menu action.
/// Keys are case-folded before dispatch (matching `input.md §6`).
/// Returns `None` for invalid keys, which the menu silently ignores.
pub fn intro_menu_action(byte: u8) -> Option<IntroMenuAction> {
    let folded = input_case_fold(byte);
    Some(match folded {
        b'J' => IntroMenuAction::JourneyOnward,
        b'C' => IntroMenuAction::CreateNewCharacter,
        b'T' => IntroMenuAction::TransferFromUltimaIv,
        b'U' => IntroMenuAction::UltimaVIntroduction,
        b'A' => IntroMenuAction::Acknowledgements,
        b'R' => IntroMenuAction::ReturnToView,
        // Enter (CR or LF) reuses the cached selection if any.
        b'\r' | b'\n' => IntroMenuAction::RepeatCachedSelection,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Locks in the clean-room first-playable defaults that stand in for
    /// not-yet-published spec values. Each row pairs the engine's
    /// current intentional clean-room value with the open spec issue
    /// that gates exact-parity ratification.
    ///
    /// If a spec issue is closed and the public spec publishes the
    /// authoritative value, update the corresponding constant *and*
    /// this test in the same patch.
    #[test]
    fn clean_room_policy_constants_match_documented_defaults() {
        // `cleak/u5-spec#49` — Create Food grant per cast is a
        // uniform `1..=3`; successful casts never grant zero food.
        assert_eq!(crate::CREATE_FOOD_MIN_GRANT, 1);
        assert_eq!(crate::CREATE_FOOD_MAX_GRANT, 3);
        // `cleak/u5-spec#50` — Hourly poison damage per Poisoned living
        // member is a deterministic `-1` (not RNG-rolled).
        assert_eq!(crate::FIRST_PLAYABLE_HOURLY_POISON_DAMAGE, 1);
        // `cleak/u5-spec#50` — Hourly starvation damage is now the
        // PRNG roll `prng_range(1, 8)` per non-dead party slot.
        assert_eq!(crate::HOURLY_STARVATION_DAMAGE_MIN, 1);
        assert_eq!(crate::HOURLY_STARVATION_DAMAGE_MAX, 8);
    }

    #[test]
    #[should_panic(expected = "forbidden fallback")]
    fn acknowledgements_contract_refuses_placeholder_lines() {
        require_acknowledgements_contract();
    }

    #[test]
    fn placeholder_title_tick_frames_satisfy_destination_contract() {
        let frames = placeholder_title_tick_frames();
        for frame in 0..TITLE_TICK_FRAME_COUNT {
            let pixels = frames.frame_pixels(frame);
            assert_eq!(pixels.len(), TITLE_TICK_FRAME_PIXELS);
            assert!(pixels.iter().all(|p| *p == 0));
        }
    }

    #[test]
    fn ega_drv_title_tick_parser_round_trips_synthesised_bands() {
        // Build a synthetic EGA.DRV-style buffer where each band's
        // first row sets every plane to 0xff (producing palette
        // index 15 across the whole row) and every other row is
        // zero. Parsing the four bands then asserts that:
        //   - row 0 of each frame is palette 15 everywhere,
        //   - rows 1..48 are palette 0 everywhere,
        //   - row 49 of the source band is *not* copied (only the
        //     upper 49 rows are taken per §5).
        let layout = EgaTitleTickLayout::standard_4_plane(0);
        let band_stride = layout.band_stride_bytes();
        let mut bytes = vec![0u8; band_stride * TITLE_TICK_FRAME_COUNT as usize];
        for band in 0..TITLE_TICK_FRAME_COUNT as usize {
            let band_base = band * band_stride;
            // Row 0: all planes set across all 40 bytes.
            for plane in 0..4 {
                let plane_base = band_base + plane * layout.plane_stride_bytes;
                for col in 0..layout.plane_stride_bytes {
                    bytes[plane_base + col] = 0xff;
                }
            }
            // Row 49 (the row §5 says is discarded): set a sentinel
            // pattern that should *not* show up in the parsed
            // frame pixels.
            let row49_base = band_base + 49 * layout.bytes_per_row;
            for plane in 0..4 {
                let plane_base = row49_base + plane * layout.plane_stride_bytes;
                for col in 0..layout.plane_stride_bytes {
                    bytes[plane_base + col] = 0xaa;
                }
            }
        }

        let frames =
            parse_ega_drv_title_tick_frames(&bytes, layout).expect("synthesised EGA.DRV decodes");
        for frame in 0..TITLE_TICK_FRAME_COUNT {
            let pixels = frames.frame_pixels(frame);
            let width = TITLE_TICK_FRAME_WIDTH as usize;
            for col in 0..width {
                assert_eq!(pixels[col], 0x0f, "frame {frame} row 0 col {col}");
            }
            for row in 1..TITLE_TICK_FRAME_HEIGHT as usize {
                for col in 0..width {
                    assert_eq!(
                        pixels[row * width + col],
                        0,
                        "frame {frame} row {row} col {col}"
                    );
                }
            }
        }
    }

    #[test]
    fn ega_drv_title_tick_parser_rejects_truncated_input() {
        let layout = EgaTitleTickLayout::standard_4_plane(0);
        let err = parse_ega_drv_title_tick_frames(&[], layout)
            .expect_err("empty buffer must fail the parser");
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn ega_drv_title_tick_parser_rejects_invalid_layout() {
        let layout = EgaTitleTickLayout {
            start_offset: 0,
            bytes_per_row: 160,
            plane_stride_bytes: 50, // 50 * 4 != 160 -> invalid
            source_rows_per_band: 50,
        };
        let buffer = vec![0u8; layout.band_stride_bytes() * 4];
        let err = parse_ega_drv_title_tick_frames(&buffer, layout)
            .expect_err("inconsistent layout must fail the parser");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
