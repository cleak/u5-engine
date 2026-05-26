//! Intro-menu key dispatch per `intro.md` §6.

use std::io;
use std::path::Path;
use std::sync::OnceLock;

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

static AUTHORED_TITLE_TICK_FRAMES: OnceLock<TitleTickFrameSet> = OnceLock::new();

pub fn authored_title_tick_frames() -> &'static TitleTickFrameSet {
    AUTHORED_TITLE_TICK_FRAMES.get_or_init(build_authored_title_tick_frames)
}

fn build_authored_title_tick_frames() -> TitleTickFrameSet {
    let mut pixels = vec![0; TITLE_TICK_FRAME_SET_BYTES];
    for frame in 0..TITLE_TICK_FRAME_COUNT {
        let (bright, dim) = title_tick_palette_indices(frame);
        let frame_start = usize::from(frame) * TITLE_TICK_FRAME_PIXELS;
        for x in 0..TITLE_TICK_FRAME_WIDTH as usize {
            let crest = title_tick_authored_wave_height(x, frame);
            let top = TITLE_TICK_FRAME_HEIGHT as usize - crest;
            for y in top..TITLE_TICK_FRAME_HEIGHT as usize {
                let local_y = y - top;
                let color = if local_y <= 7 || title_tick_authored_highlight(x, y, frame) {
                    bright
                } else {
                    dim
                };
                pixels[frame_start + y * TITLE_TICK_FRAME_WIDTH as usize + x] = color;
            }
        }
    }
    TitleTickFrameSet::from_palette_indices(pixels, "authored clean-room title tick frames")
        .expect("authored title-tick frame generator emitted invalid EGA palette data")
}

fn title_tick_authored_wave_height(x: usize, frame: u8) -> usize {
    let phase = usize::from(frame);
    let broad = triangle_wave((x + phase * 13) % 96, 96);
    let fine = triangle_wave((x * 3 + phase * 17) % 47, 47);
    let notch = triangle_wave((x * 5 + phase * 11) % 31, 31);
    let height = 12 + broad / 3 + fine / 5 - notch / 10;
    height.clamp(8, TITLE_TICK_FRAME_HEIGHT as usize)
}

fn title_tick_authored_highlight(x: usize, y: usize, frame: u8) -> bool {
    ((x + y * 2 + usize::from(frame) * 9) % 23) <= 2
}

fn triangle_wave(position: usize, period: usize) -> usize {
    assert!(period >= 2, "triangle wave period must be at least two");
    let half = period / 2;
    if position <= half {
        position
    } else {
        period - position
    }
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

/// `intro.md §11`: lines shown by the Acknowledgements (`A`) submenu.
///
/// The public spec calls out acknowledgement-screen content as
/// clean-room authored: "Its exact text and pagination are left to a
/// source-free content transcription rather than copied binary text
/// dumps." This is original prose authored for the clean-room
/// implementation; it does not transcribe any historical credit text.
pub const ACKNOWLEDGEMENTS_LINES: &[&str] = &[
    "Acknowledgements",
    "",
    "Ultima V is a trademark of its",
    "rights holders.",
    "This clean-room recreation reads",
    "local game data at runtime and",
    "ships no original game content.",
    "",
    "Behavior is derived from the",
    "published clean-room specification,",
    "this engine, and local game assets.",
    "No private decompilation,",
    "disassembly, or copyrighted source",
    "has been consulted.",
    "",
    "Key: return to the intro menu.",
];

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
    fn acknowledgements_lines_are_clean_room_authored() {
        let header = ACKNOWLEDGEMENTS_LINES
            .first()
            .copied()
            .expect("acknowledgements has at least a header line");
        assert_eq!(header, "Acknowledgements");
        let last = ACKNOWLEDGEMENTS_LINES
            .last()
            .copied()
            .expect("acknowledgements has a closing prompt");
        assert!(
            last.contains("return to the intro menu"),
            "closing line should prompt return to the intro menu, got `{last}`"
        );
        assert!(
            ACKNOWLEDGEMENTS_LINES
                .iter()
                .any(|line| line.contains("clean-room")),
            "acknowledgements text should describe its clean-room provenance"
        );
        assert!(
            ACKNOWLEDGEMENTS_LINES.iter().all(|line| line.len() <= 36),
            "every acknowledgement line must fit the 36-column intro box"
        );
    }
}
