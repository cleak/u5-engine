//! Intro-menu key dispatch per `intro.md` §6.

use std::io;
use std::path::Path;

use crate::{
    GraphicImage, GraphicImageDirectory, TileGraphicsDepth, input_case_fold,
    load_graphic_image_directory, read_optional_disk_file,
};

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
/// Runtime observation of the original's lower intro menu frame,
/// pending the spec correction tracked as `cleak/u5-spec#78`.
///
/// `systems/intro.md §6.1` describes this frame as a single-line
/// rectangle built from five reserved box-drawing glyphs in the
/// intro's bright foreground index. A black-box capture of the
/// original shows instead the same rounded blue chrome the gameplay
/// border uses: pixel rows 120..=199 filled with EGA index 1 behind a
/// 1-pixel index-15 rectangle, a black interior, and two captions
/// drawn over the border rows.
pub const INTRO_MENU_FRAME_BORDER_COLOR: u8 = 0x01;
pub const INTRO_MENU_FRAME_OUTLINE_COLOR: u8 = 0x0f;
pub const INTRO_MENU_FRAME_INTERIOR_COLOR: u8 = 0x00;
/// First and last pixel row of the blue border band. Anchored to the
/// published `§6.1` cell rectangle: rows 15..=24 of the 8-pixel text
/// grid.
pub const INTRO_MENU_FRAME_TOP_Y: u16 = INTRO_MENU_FRAME_ANCHOR_ROW as u16 * 8;
pub const INTRO_MENU_FRAME_BOTTOM_Y: u16 =
    INTRO_MENU_FRAME_TOP_Y + INTRO_MENU_FRAME_HEIGHT_CELLS as u16 * 8 - 1;
/// `cleak/u5-spec#78` corner-rounding profile: the left-edge column
/// at which the blue fill starts, for each of the first six rows of
/// the band. Rows past the profile start at column 0, and the bottom
/// six rows mirror the profile in reverse. The right edge mirrors
/// each entry about the surface centre.
///
/// The gameplay border frame carves its outer corners with the same
/// measured staircase, so the numbers live once in
/// [`crate::gameplay_chrome::CHROME_CORNER_PROFILE`] and this name
/// stays as the `§6.1`-facing alias.
pub const INTRO_MENU_FRAME_CORNER_PROFILE: [u16; 6] = crate::gameplay_chrome::CHROME_CORNER_PROFILE;
/// `intro.md §6.1` horizontal-rule pixel coordinates. Observation
/// confirms the published top rule and adds the matching bottom rule
/// plus the two verticals that close the rectangle.
pub const INTRO_MENU_FRAME_BOTTOM_RULE_Y: u16 = 192;
pub const INTRO_MENU_FRAME_OUTLINE_LEFT_X: u16 = INTRO_MENU_FRAME_RULE_X0;
pub const INTRO_MENU_FRAME_OUTLINE_RIGHT_X: u16 = INTRO_MENU_FRAME_RULE_X1;
pub const INTRO_MENU_FRAME_INTERIOR_TOP_Y: u16 = INTRO_MENU_FRAME_RULE_Y + 1;
pub const INTRO_MENU_FRAME_INTERIOR_BOTTOM_Y: u16 = INTRO_MENU_FRAME_BOTTOM_RULE_Y - 1;
pub const INTRO_MENU_FRAME_INTERIOR_LEFT_X: u16 = INTRO_MENU_FRAME_OUTLINE_LEFT_X + 1;
pub const INTRO_MENU_FRAME_INTERIOR_RIGHT_X: u16 = INTRO_MENU_FRAME_OUTLINE_RIGHT_X - 1;

/// `cleak/u5-spec#78` border captions. Both are drawn as ordinary
/// white-on-black fixed cells over the blue border rows and visibly
/// interrupt the white rules, exactly like the gameplay border's wind
/// label.
pub const INTRO_MENU_SELECT_CAPTION_PREFIX: &str = ">Select:";
pub const INTRO_MENU_SELECT_CAPTION_SUFFIX: &str = "<";
/// The caption's cursor cell is `IBM.CH` glyph 8 (a diagonal hatch),
/// measured directly from the capture's cell 23 of text row 15. The
/// engine's gameplay prompt cursor (`PROMPT_CURSOR_GLYPH`) is a
/// different glyph, so this caption names its own code rather than
/// reusing it.
pub const INTRO_MENU_SELECT_CAPTION_CURSOR_GLYPH: u8 =
    crate::gameplay_chrome::PROMPT_CURSOR_FRAME_GLYPHS[3];
pub const INTRO_MENU_SELECT_CAPTION_COLUMN: u8 = 15;
pub const INTRO_MENU_SELECT_CAPTION_ROW: u8 = 15;
pub const INTRO_MENU_COPYRIGHT_CAPTION: &str = ">Copyright 1988 Lord British<";
pub const INTRO_MENU_COPYRIGHT_CAPTION_COLUMN: u8 = 5;
pub const INTRO_MENU_COPYRIGHT_CAPTION_ROW: u8 = 24;

/// `cleak/u5-spec#78`: the left-edge column at which the blue border
/// fill starts on `row`, or `None` when the row is outside the band.
pub fn intro_menu_frame_border_start_column(row: u16) -> Option<u16> {
    if row < INTRO_MENU_FRAME_TOP_Y || row > INTRO_MENU_FRAME_BOTTOM_Y {
        return None;
    }
    let profile_len = INTRO_MENU_FRAME_CORNER_PROFILE.len() as u16;
    let from_top = row - INTRO_MENU_FRAME_TOP_Y;
    let from_bottom = INTRO_MENU_FRAME_BOTTOM_Y - row;
    if from_top < profile_len {
        Some(INTRO_MENU_FRAME_CORNER_PROFILE[from_top as usize])
    } else if from_bottom < profile_len {
        Some(INTRO_MENU_FRAME_CORNER_PROFILE[from_bottom as usize])
    } else {
        Some(0)
    }
}

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
/// title-tick path draws one frame strip over the title screen at
/// this fixed pixel rectangle, then advances the frame index modulo
/// four. The destination rectangle is the published 320-by-49 band
/// at `(0, 65)`; the source pixels only cover its central 288
/// columns (see [`TITLE_TICK_SOURCE_X`]).
pub const TITLE_TICK_FRAME_X: u16 = 0;
pub const TITLE_TICK_FRAME_Y: u16 = 65;
pub const TITLE_TICK_FRAME_WIDTH: u16 = TITLE_SURFACE_WIDTH;
pub const TITLE_TICK_FRAME_HEIGHT: u16 = 49;
pub const TITLE_TICK_FRAME_COUNT: u8 = 4;

/// Runtime observation of the shipped `ULTIMA.16` asset, pending the
/// spec correction tracked as `cleak/u5-spec#78`.
///
/// `systems/intro.md §5` and `systems/display-driver.md §8` claim the
/// four flaming "Warriors of Destiny" bands live only inside the EGA
/// driver's runtime back-buffer and cannot be read from an external
/// art file. Decoding the local `ULTIMA.16` image directory
/// contradicts that: it carries five panels — slot 0 is the 319-by-61
/// "Ultima V" logo and slots 1..=4 are the four 288-wide title-tick
/// bands (three at 49 rows, the last at 50). A black-box capture of
/// the original running the same assets matches those panels exactly
/// at `(0, 0)` and `(16, 65)` respectively, so the engine renders from
/// the asset rather than from authored replacement art.
pub const ULTIMA_PANEL_STEM: &str = "ULTIMA";
/// `cleak/u5-spec#78`: `ULTIMA` slot 0 is the menu logo, blitted at
/// the surface origin.
pub const ULTIMA_LOGO_SLOT: u8 = 0;
pub const ULTIMA_LOGO_WIDTH: usize = 319;
pub const ULTIMA_LOGO_HEIGHT: usize = 61;
/// `cleak/u5-spec#78`: `ULTIMA` slots 1..=4 are title-tick frames
/// 0..=3. Frame 0 is slot 1 — the slot-to-frame mapping is assumed to
/// be the natural directory order because the captures only pin slots
/// 2 and 3 (which match the settled menu), and the four-frame loop
/// makes any rotation visually equivalent after one cycle.
pub const ULTIMA_TITLE_TICK_FIRST_SLOT: u8 = 1;
/// `cleak/u5-spec#78`: horizontal offset of the 288-wide source band
/// inside the published 320-wide destination rectangle. Columns
/// `0..=15` and `304..=319` of the destination rows are cleared to
/// palette index 0 on every tick.
pub const TITLE_TICK_SOURCE_X: u16 = 16;
pub const TITLE_TICK_SOURCE_WIDTH: u16 = 288;
/// `cleak/u5-spec#78`: `ULTIMA` slot 4 is authored with a 50th row
/// that the destination rectangle does not consume; only the upper
/// [`TITLE_TICK_FRAME_HEIGHT`] rows of each panel are copied, which
/// corroborates the published "50-row source stride, upper 49 rows
/// copied" rule.
pub const TITLE_TICK_SOURCE_MAX_HEIGHT: usize = 50;

pub const TITLE_TICK_FRAME_PIXELS: usize =
    TITLE_TICK_SOURCE_WIDTH as usize * TITLE_TICK_FRAME_HEIGHT as usize;
pub const TITLE_TICK_FRAME_SET_BYTES: usize =
    TITLE_TICK_FRAME_PIXELS * TITLE_TICK_FRAME_COUNT as usize;

/// `intro.md §5`: advance the title-tick frame index modulo four.
pub const fn title_tick_next_frame(current_frame: u8) -> u8 {
    (current_frame + 1) % TITLE_TICK_FRAME_COUNT
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
                    "{source}: title-tick frame set must be exactly {TITLE_TICK_FRAME_SET_BYTES} bytes ({} frames of {}x{}), found {}",
                    TITLE_TICK_FRAME_COUNT,
                    TITLE_TICK_SOURCE_WIDTH,
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

/// `cleak/u5-spec#78` title-tick source loader. Reads the local
/// `ULTIMA` image directory and emits the four-frame strip from slots
/// [`ULTIMA_TITLE_TICK_FIRST_SLOT`]..=`+3`, taking the upper
/// [`TITLE_TICK_FRAME_HEIGHT`] rows of each 288-wide panel.
///
/// This is a runtime read of a local asset, not authored replacement
/// art: the panels are the original flaming "Warriors of Destiny"
/// bands, and a black-box capture of the original confirms them at
/// `(16, 65)`.
pub fn load_ultima_title_tick_frames(
    game_dir: &Path,
    depth: TileGraphicsDepth,
) -> io::Result<TitleTickFrameSet> {
    let directory = load_graphic_image_directory(game_dir, ULTIMA_PANEL_STEM, depth)?;
    parse_ultima_title_tick_frames(&directory)
}

/// `cleak/u5-spec#78`: extract the four title-tick frames from an
/// already-decoded `ULTIMA` image directory.
pub fn parse_ultima_title_tick_frames(
    directory: &GraphicImageDirectory,
) -> io::Result<TitleTickFrameSet> {
    let width = TITLE_TICK_SOURCE_WIDTH as usize;
    let height = TITLE_TICK_FRAME_HEIGHT as usize;
    let mut pixels = Vec::with_capacity(TITLE_TICK_FRAME_SET_BYTES);
    for frame in 0..TITLE_TICK_FRAME_COUNT {
        let slot = usize::from(ULTIMA_TITLE_TICK_FIRST_SLOT + frame);
        let panel = directory
            .images
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{ULTIMA_PANEL_STEM} image directory is missing title-tick panel slot {slot}"
                    ),
                )
            })?;
        if panel.width != width
            || panel.height < height
            || panel.height > TITLE_TICK_SOURCE_MAX_HEIGHT
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{ULTIMA_PANEL_STEM} title-tick panel slot {slot} is {}x{}, expected {width} wide and {height}..={TITLE_TICK_SOURCE_MAX_HEIGHT} rows tall",
                    panel.width, panel.height
                ),
            ));
        }
        // §5 / `cleak/u5-spec#78`: only the upper `height` rows of the
        // source band reach the destination rectangle.
        pixels.extend_from_slice(&panel.pixels[..width * height]);
    }
    TitleTickFrameSet::from_palette_indices(pixels, "ULTIMA title-tick panels")
}

/// `cleak/u5-spec#78` menu-logo loader. Reads `ULTIMA` slot 0, the
/// 319-by-61 "Ultima V" logo the original blits at the surface origin
/// on the start/menu screen.
pub fn load_ultima_logo_panel(
    game_dir: &Path,
    depth: TileGraphicsDepth,
) -> io::Result<GraphicImage> {
    let directory = load_graphic_image_directory(game_dir, ULTIMA_PANEL_STEM, depth)?;
    let panel = directory
        .images
        .get(usize::from(ULTIMA_LOGO_SLOT))
        .and_then(Option::as_ref)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{ULTIMA_PANEL_STEM} image directory is missing logo slot {ULTIMA_LOGO_SLOT}"
                ),
            )
        })?;
    if (panel.width, panel.height) != (ULTIMA_LOGO_WIDTH, ULTIMA_LOGO_HEIGHT) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{ULTIMA_PANEL_STEM} logo slot {ULTIMA_LOGO_SLOT} is {}x{}, expected {ULTIMA_LOGO_WIDTH}x{ULTIMA_LOGO_HEIGHT}",
                panel.width, panel.height
            ),
        ));
    }
    Ok(panel.clone())
}

/// Four-frame strip of all-zero (black) pixels satisfying the public
/// destination contract (288×49 per frame, four frames, in-range EGA
/// palette indices). Test-only scaffolding for the destination-blit
/// geometry; no render path uses it.
pub fn placeholder_title_tick_frames() -> TitleTickFrameSet {
    TitleTickFrameSet::from_palette_indices(
        vec![0u8; TITLE_TICK_FRAME_SET_BYTES],
        "placeholder title-tick frames",
    )
    .expect("placeholder title-tick frame set is well-formed by construction")
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
/// `intro.md §11` acknowledgements gate for callers with no graphical
/// surface.
///
/// §11 describes an artwork screen, not a text screen: a graphics
/// resource drawn at a fixed origin, revealed by a bottom-up slab
/// wipe, held for a keypress, wiped away top-to-bottom, then `STARTSC`
/// reloaded and the menu repainted. The graphical intro draws that
/// artwork; the terminal harness cannot, and printing
/// clean-room-authored credit lines in its place would be inventing
/// the one thing §15 reserves for a source-free transcription.
///
/// Still unimplemented on the graphical side, and still the reason
/// `cleak/u5-spec#72` is open: the slab height / pixel stride and the
/// per-slab tick cadence of the entry and exit wipes.
pub fn require_acknowledgements_contract() -> ! {
    panic!(
        "intro acknowledgements are the §11 credits artwork screen, which needs the graphical intro renderer; the terminal harness has no surface to draw it on, and substituting clean-room-authored placeholder credits is a forbidden fallback. The entry/exit slab wipe stride and cadence are also still unpublished; see cleak/u5-spec#72"
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

    /// `cleak/u5-spec#78`: the shipped `ULTIMA` image directory holds
    /// five panels - the 319x61 menu logo followed by the four
    /// title-tick bands (288x49, 288x49, 288x49, 288x50). The tests
    /// read the local clean asset directory when it is present and
    /// skip when it is not, so a checkout without game files still
    /// passes.
    fn local_ultima_directory() -> Option<GraphicImageDirectory> {
        let game_dir = Path::new(crate::DEFAULT_GAME_DIR);
        if !game_dir
            .join(crate::tile_graphics_file_name(
                ULTIMA_PANEL_STEM,
                TileGraphicsDepth::Ega16,
            ))
            .exists()
        {
            return None;
        }
        Some(
            load_graphic_image_directory(game_dir, ULTIMA_PANEL_STEM, TileGraphicsDepth::Ega16)
                .expect("local ULTIMA image directory decodes"),
        )
    }

    #[test]
    fn local_ultima_directory_has_the_published_five_panel_shape() {
        let Some(directory) = local_ultima_directory() else {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        };
        let shapes: Vec<(usize, usize)> = directory
            .images
            .iter()
            .map(|image| {
                let image = image.as_ref().expect("ULTIMA panels are all populated");
                (image.width, image.height)
            })
            .collect();
        assert_eq!(
            shapes,
            vec![(319, 61), (288, 49), (288, 49), (288, 49), (288, 50)],
            "ULTIMA slot 0 is the menu logo and slots 1..=4 are the title-tick bands"
        );
    }

    #[test]
    fn ultima_title_tick_panels_fill_the_288_by_49_frame_buffer() {
        let Some(directory) = local_ultima_directory() else {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        };
        let frames =
            parse_ultima_title_tick_frames(&directory).expect("ULTIMA title-tick panels decode");
        assert_eq!(
            TITLE_TICK_FRAME_PIXELS,
            TITLE_TICK_SOURCE_WIDTH as usize * TITLE_TICK_FRAME_HEIGHT as usize
        );
        for frame in 0..TITLE_TICK_FRAME_COUNT {
            let pixels = frames.frame_pixels(frame);
            assert_eq!(
                pixels.len(),
                TITLE_TICK_FRAME_PIXELS,
                "frame {frame} is {}x{}",
                TITLE_TICK_SOURCE_WIDTH,
                TITLE_TICK_FRAME_HEIGHT
            );
            assert!(pixels.iter().all(|index| *index <= 0x0f));
            assert!(
                pixels.iter().any(|index| *index != 0),
                "frame {frame} must carry the flaming band, not an empty strip"
            );
        }
    }

    #[test]
    fn ultima_title_tick_panels_take_only_the_upper_rows_of_the_50_row_band() {
        let Some(directory) = local_ultima_directory() else {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        };
        let frames =
            parse_ultima_title_tick_frames(&directory).expect("ULTIMA title-tick panels decode");
        let last_slot = usize::from(ULTIMA_TITLE_TICK_FIRST_SLOT + TITLE_TICK_FRAME_COUNT - 1);
        let panel = directory.images[last_slot]
            .as_ref()
            .expect("last title-tick panel is populated");
        assert_eq!(panel.height, TITLE_TICK_SOURCE_MAX_HEIGHT);
        let width = TITLE_TICK_SOURCE_WIDTH as usize;
        let height = TITLE_TICK_FRAME_HEIGHT as usize;
        assert_eq!(
            frames.frame_pixels(TITLE_TICK_FRAME_COUNT - 1),
            &panel.pixels[..width * height],
            "only the upper {height} rows of the 50-row source band reach the destination"
        );
    }

    #[test]
    fn ultima_logo_panel_is_the_published_319_by_61_menu_art() {
        let game_dir = Path::new(crate::DEFAULT_GAME_DIR);
        if local_ultima_directory().is_none() {
            eprintln!("skipping: local ULTIMA.16 is not present");
            return;
        }
        let logo = load_ultima_logo_panel(game_dir, TileGraphicsDepth::Ega16)
            .expect("local ULTIMA logo panel decodes");
        assert_eq!(
            (logo.width, logo.height),
            (ULTIMA_LOGO_WIDTH, ULTIMA_LOGO_HEIGHT)
        );
        assert_eq!(logo.pixels.len(), ULTIMA_LOGO_WIDTH * ULTIMA_LOGO_HEIGHT);
    }

    #[test]
    fn ultima_title_tick_parser_rejects_a_directory_without_the_bands() {
        let directory = GraphicImageDirectory {
            depth: TileGraphicsDepth::Ega16,
            images: vec![None],
        };
        let err = parse_ultima_title_tick_frames(&directory)
            .expect_err("a directory without title-tick panels must fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn intro_menu_frame_border_profile_rounds_both_ends_and_fills_the_middle() {
        // `cleak/u5-spec#78` measured profile: the blue fill starts at
        // column 5 on the first band row, 3, 2, 1, 1, then 0 from the
        // sixth row on, and the bottom six rows mirror it.
        assert_eq!(intro_menu_frame_border_start_column(119), None);
        assert_eq!(intro_menu_frame_border_start_column(120), Some(5));
        assert_eq!(intro_menu_frame_border_start_column(121), Some(3));
        assert_eq!(intro_menu_frame_border_start_column(122), Some(2));
        assert_eq!(intro_menu_frame_border_start_column(123), Some(1));
        assert_eq!(intro_menu_frame_border_start_column(124), Some(1));
        for y in 125..=194 {
            assert_eq!(intro_menu_frame_border_start_column(y), Some(0), "row {y}");
        }
        assert_eq!(intro_menu_frame_border_start_column(195), Some(1));
        assert_eq!(intro_menu_frame_border_start_column(196), Some(1));
        assert_eq!(intro_menu_frame_border_start_column(197), Some(2));
        assert_eq!(intro_menu_frame_border_start_column(198), Some(3));
        assert_eq!(intro_menu_frame_border_start_column(199), Some(5));
        assert_eq!(intro_menu_frame_border_start_column(200), None);
    }

    #[test]
    fn intro_menu_frame_rectangle_matches_the_measured_geometry() {
        assert_eq!(INTRO_MENU_FRAME_TOP_Y, 120);
        assert_eq!(INTRO_MENU_FRAME_BOTTOM_Y, 199);
        assert_eq!(INTRO_MENU_FRAME_RULE_Y, 127);
        assert_eq!(INTRO_MENU_FRAME_BOTTOM_RULE_Y, 192);
        assert_eq!(INTRO_MENU_FRAME_RULE_X0, 7);
        assert_eq!(INTRO_MENU_FRAME_RULE_X1, 312);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_LEFT_X, 8);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_RIGHT_X, 311);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_TOP_Y, 128);
        assert_eq!(INTRO_MENU_FRAME_INTERIOR_BOTTOM_Y, 191);
        assert_eq!(INTRO_MENU_FRAME_BORDER_COLOR, 0x01);
        assert_eq!(INTRO_MENU_FRAME_OUTLINE_COLOR, 0x0f);
    }

    #[test]
    fn intro_menu_border_captions_occupy_the_measured_cells() {
        // `>Select:` + cursor + `<` fills cells 15..=24 of row 15;
        // the copyright caption fills cells 5..=33 of row 24.
        let select_cells =
            INTRO_MENU_SELECT_CAPTION_PREFIX.len() + 1 + INTRO_MENU_SELECT_CAPTION_SUFFIX.len();
        assert_eq!(select_cells, 10);
        assert_eq!(INTRO_MENU_SELECT_CAPTION_COLUMN, 15);
        assert_eq!(
            usize::from(INTRO_MENU_SELECT_CAPTION_COLUMN) + select_cells - 1,
            24
        );
        assert_eq!(INTRO_MENU_SELECT_CAPTION_ROW, 15);
        assert_eq!(INTRO_MENU_COPYRIGHT_CAPTION.len(), 29);
        assert_eq!(INTRO_MENU_COPYRIGHT_CAPTION_COLUMN, 5);
        assert_eq!(
            usize::from(INTRO_MENU_COPYRIGHT_CAPTION_COLUMN) + INTRO_MENU_COPYRIGHT_CAPTION.len()
                - 1,
            33
        );
        assert_eq!(INTRO_MENU_COPYRIGHT_CAPTION_ROW, 24);
    }
}
