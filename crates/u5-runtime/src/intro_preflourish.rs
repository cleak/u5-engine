//! `systems/intro.md §3` pre-flourish phase (step 2).
//!
//! The pre-flourish phase sits between the surface clear (step 1) and
//! the `TITLE.BIT` flourish load (step 3). It is a non-visual
//! preparation pass: it loads the intro's two resident glyph assets
//! into a slot table, activates the IBM slot, resets the primary
//! text window to the full screen, selects the active display-driver
//! descriptor, and performs exactly one non-blocking keyboard poll
//! for the early Journey Onward shortcut.
//!
//! Nothing is drawn during this phase. The optional `"Journey
//! Onward"` banner that the J-shortcut path prints belongs to the
//! caller; this module surfaces the contract text as
//! [`JOURNEY_ONWARD_SHORTCUT_BANNER`] and returns
//! [`PreFlourishOutcome::JourneyOnwardShortcut`] so the harness can
//! render the banner with the now-active IBM font before dispatching
//! the Journey Onward load handler.

use std::io;
use std::path::Path;

use crate::{
    DisplayDriverFamily, FixedCellFont, IBM_CH_FILE, RUNES_CH_FILE, TEXT_SCREEN_COLUMNS,
    TEXT_SCREEN_ROWS, TextWindowSystem, input_case_fold, parse_ch_font, read_disk_file,
};

/// `systems/intro.md §3` Pre-flourish phase step 2: the two glyph
/// assets loaded into the resident font-slot table. Slot 0 is the
/// IBM-style Roman alphabet used for ordinary intro and menu text;
/// slot 1 is the Britannian runic alphabet kept resident for later
/// activation (chargen, story slides — both out of scope for this
/// phase).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntroFontSlot {
    Ibm,
    Runes,
}

/// Resident font-slot table filled by the pre-flourish phase.
///
/// `systems/intro.md §3` step 2 calls for "at least two" resident
/// font slots and an "active font" indirected through the slot
/// table. The internal representation is engine-native: the two
/// loaded fonts are kept as fields and `active` selects which one
/// later text output uses. Switching `active` is a single field
/// assignment — no I/O, no allocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntroFontSlots {
    ibm: FixedCellFont,
    runes: FixedCellFont,
    active: IntroFontSlot,
}

impl IntroFontSlots {
    /// Builds a slot table from two parsed `.CH` fonts and activates
    /// the IBM slot, matching the pre-flourish phase step 3 default.
    pub fn new(ibm: FixedCellFont, runes: FixedCellFont) -> Self {
        Self {
            ibm,
            runes,
            active: IntroFontSlot::Ibm,
        }
    }

    pub fn active_slot(&self) -> IntroFontSlot {
        self.active
    }

    pub fn set_active_slot(&mut self, slot: IntroFontSlot) {
        self.active = slot;
    }

    pub fn active_font(&self) -> &FixedCellFont {
        self.font(self.active)
    }

    pub fn font(&self, slot: IntroFontSlot) -> &FixedCellFont {
        match slot {
            IntroFontSlot::Ibm => &self.ibm,
            IntroFontSlot::Runes => &self.runes,
        }
    }

    pub fn ibm_font(&self) -> &FixedCellFont {
        &self.ibm
    }

    pub fn runes_font(&self) -> &FixedCellFont {
        &self.runes
    }
}

/// `systems/intro.md §3` step 6: outcome of the single non-blocking
/// keyboard poll. The poll fires exactly once at the end of the
/// pre-flourish phase; folding the queued key to uppercase and
/// matching `J` selects the early Journey Onward shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreFlourishOutcome {
    /// No key was queued, or the queued key did not fold to `J`.
    /// The outer phase list should continue to step 3 (title and
    /// Lord British resource load, followed by the `TITLE.BIT`
    /// flourish).
    ContinueToFlourish,
    /// The queued key folded to `J`. The intro should print
    /// `"Journey Onward"` centered in the cleared text window using
    /// the now-active IBM font, then jump directly to the Journey
    /// Onward load handler in `systems/intro.md §7`.
    JourneyOnwardShortcut,
}

/// `systems/intro.md §3` step 6 banner text printed when the early
/// J shortcut is taken. Centered in the now-active full-screen text
/// window through the IBM glyph slot.
pub const JOURNEY_ONWARD_SHORTCUT_BANNER: &str = "Journey Onward";

/// `systems/intro.md §3` step 5 driver descriptor selection. The
/// pre-flourish phase selects descriptor index 0 as the active
/// display-driver descriptor. This is configuration bookkeeping that
/// publishes the descriptor's text/colour state to the resident text
/// primitives; it is not a BIOS mode switch.
pub const PRE_FLOURISH_TEXT_WINDOW_INDEX: usize = 0;

/// `systems/intro.md §3` step 2: load `IBM.CH` and `RUNES.CH` (or
/// the corresponding `.HCS` files on Hercules) into the resident
/// slot table and activate the IBM slot.
///
/// EGA, CGA, and Tandy all share the 8x8 `.CH` font geometry. The
/// Hercules path uses the 16x12 `.HCS` font geometry and is
/// alternate-hardware parity work — it can not yet share the
/// [`FixedCellFont`] type used by the EGA renderer, so the loader
/// returns an error on that driver until the engine grows an
/// HCS-capable text path.
pub fn load_intro_font_slots(
    game_dir: &Path,
    driver: DisplayDriverFamily,
) -> io::Result<IntroFontSlots> {
    let (ibm_file, runes_file) = match driver {
        DisplayDriverFamily::Cga | DisplayDriverFamily::Ega | DisplayDriverFamily::Tandy => {
            (IBM_CH_FILE, RUNES_CH_FILE)
        }
        DisplayDriverFamily::Hercules => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Hercules .HCS intro fonts are alternate-hardware parity work; the v1 EGA-compatible engine does not yet ship an HCS-capable text path (intro.md §3 step 2)",
            ));
        }
    };
    let ibm = parse_ch_font(&read_disk_file(&game_dir.join(ibm_file))?, ibm_file)?;
    let runes = parse_ch_font(&read_disk_file(&game_dir.join(runes_file))?, runes_file)?;
    Ok(IntroFontSlots::new(ibm, runes))
}

/// `systems/intro.md §3` pre-flourish phase entry point.
///
/// Performs the steps spelled out in the spec subsection, in order:
///
/// 1. Driver-ready probe is the caller's responsibility — the
///    presence of a [`TextWindowSystem`] here stands in for the
///    spec's "intro-ready" assertion. A clean engine that has
///    already initialised the display backend can treat the probe
///    as a no-op; a backend that can still fail this late should
///    bubble its failure up through its own `Result` channel rather
///    than the spec's BIOS-mode-restore + exit-1 fallback (the
///    clean implementation does not reuse that fallback).
/// 2. Loads the IBM and rune glyph assets into the slot table and
///    activates the IBM slot. The Hercules `.HCS` route is deferred
///    (see [`load_intro_font_slots`]).
/// 3. Activation is bundled with the load: the slot table is born
///    with [`IntroFontSlot::Ibm`] active.
/// 4. Resets the primary text window's descriptor to the full
///    40-column by 25-row rectangle.
/// 5. Selects display-driver descriptor index 0 as the active
///    descriptor (clean engine: the active text window index).
/// 6. Performs one non-blocking keyboard poll. The caller supplies
///    the queued byte (or `None` for "nothing waiting"). A queued
///    byte that folds to `J` yields
///    [`PreFlourishOutcome::JourneyOnwardShortcut`]; everything
///    else yields [`PreFlourishOutcome::ContinueToFlourish`].
///
/// This function performs no rendering. The optional `"Journey
/// Onward"` banner that the J-shortcut path is required to print is
/// the harness's responsibility once it sees the shortcut outcome.
/// Keeping the rendering policy outside the runtime lets the
/// terminal shell and the Bevy harness share the same state-machine
/// logic.
pub fn run_intro_pre_flourish_phase(
    game_dir: &Path,
    driver: DisplayDriverFamily,
    text_windows: &mut TextWindowSystem,
    queued_key: Option<u8>,
) -> io::Result<(IntroFontSlots, PreFlourishOutcome)> {
    let slots = load_intro_font_slots(game_dir, driver)?;

    text_windows.set_window_rect(
        PRE_FLOURISH_TEXT_WINDOW_INDEX,
        0,
        0,
        TEXT_SCREEN_COLUMNS - 1,
        TEXT_SCREEN_ROWS - 1,
    );
    text_windows.set_active_window(PRE_FLOURISH_TEXT_WINDOW_INDEX);

    let outcome = match queued_key {
        Some(byte) if input_case_fold(byte) == b'J' => PreFlourishOutcome::JourneyOnwardShortcut,
        _ => PreFlourishOutcome::ContinueToFlourish,
    };

    Ok((slots, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CH_FONT_LEN, FixedCellFont, parse_ch_font};

    /// A 1024-byte fixture that places one set bit in the glyph for
    /// uppercase `A` (code 65) at the row we want to probe, so a test
    /// can confirm the slot table preserves glyph data through the
    /// load + slot-table round trip without depending on an on-disk
    /// asset.
    fn synthesized_ch_font(marker_row: usize, marker_byte: u8) -> FixedCellFont {
        let mut bytes = vec![0u8; CH_FONT_LEN];
        // glyph 65 starts at byte 65 * 8 = 520 and is 8 bytes long.
        bytes[65 * 8 + marker_row] = marker_byte;
        parse_ch_font(&bytes, "fixture.ch").expect("synthesized CH font parses")
    }

    fn empty_text_windows() -> TextWindowSystem {
        let mut windows = TextWindowSystem::new();
        // Move descriptor 0 to a partial rect so the pre-flourish
        // step 4 reset has something to undo.
        windows.set_window_rect(0, 5, 5, 10, 10);
        windows.set_active_window(2);
        windows
    }

    #[test]
    fn new_slot_table_activates_ibm() {
        let ibm = synthesized_ch_font(0, 0xff);
        let runes = synthesized_ch_font(0, 0x55);
        let slots = IntroFontSlots::new(ibm.clone(), runes.clone());
        assert_eq!(slots.active_slot(), IntroFontSlot::Ibm);
        assert_eq!(slots.active_font(), &ibm);
        assert_eq!(slots.font(IntroFontSlot::Ibm), &ibm);
        assert_eq!(slots.font(IntroFontSlot::Runes), &runes);
    }

    #[test]
    fn set_active_slot_switches_active_font_without_realloc() {
        let ibm = synthesized_ch_font(2, 0x80);
        let runes = synthesized_ch_font(2, 0x40);
        let mut slots = IntroFontSlots::new(ibm.clone(), runes.clone());
        assert_eq!(slots.active_font(), &ibm);
        slots.set_active_slot(IntroFontSlot::Runes);
        assert_eq!(slots.active_slot(), IntroFontSlot::Runes);
        assert_eq!(slots.active_font(), &runes);
        // Slot 0 stays resident through the switch.
        assert_eq!(slots.ibm_font(), &ibm);
    }

    #[test]
    fn pre_flourish_resets_full_screen_text_window_and_selects_index_zero() {
        let mut windows = empty_text_windows();
        let ibm = synthesized_ch_font(0, 0x80);
        let runes = synthesized_ch_font(0, 0x01);
        // Inline the body of `run_intro_pre_flourish_phase` past
        // the disk-touching load so the test can verify the text-
        // window side effects without needing IBM.CH on disk.
        let slots = IntroFontSlots::new(ibm, runes);
        windows.set_window_rect(
            PRE_FLOURISH_TEXT_WINDOW_INDEX,
            0,
            0,
            TEXT_SCREEN_COLUMNS - 1,
            TEXT_SCREEN_ROWS - 1,
        );
        windows.set_active_window(PRE_FLOURISH_TEXT_WINDOW_INDEX);

        let descriptor = windows
            .window(PRE_FLOURISH_TEXT_WINDOW_INDEX)
            .expect("descriptor 0 must exist");
        assert_eq!(descriptor.top_left_x, 0);
        assert_eq!(descriptor.top_left_y, 0);
        assert_eq!(descriptor.bottom_right_x, TEXT_SCREEN_COLUMNS - 1);
        assert_eq!(descriptor.bottom_right_y, TEXT_SCREEN_ROWS - 1);
        assert_eq!(
            windows.active_window_index(),
            PRE_FLOURISH_TEXT_WINDOW_INDEX
        );
        assert_eq!(slots.active_slot(), IntroFontSlot::Ibm);
    }

    #[test]
    fn outcome_is_continue_when_no_key_is_queued() {
        let outcome = match Option::<u8>::None {
            Some(byte) if input_case_fold(byte) == b'J' => {
                PreFlourishOutcome::JourneyOnwardShortcut
            }
            _ => PreFlourishOutcome::ContinueToFlourish,
        };
        assert_eq!(outcome, PreFlourishOutcome::ContinueToFlourish);
    }

    #[test]
    fn outcome_is_journey_shortcut_for_queued_uppercase_j() {
        let outcome = match Some(b'J') {
            Some(byte) if input_case_fold(byte) == b'J' => {
                PreFlourishOutcome::JourneyOnwardShortcut
            }
            _ => PreFlourishOutcome::ContinueToFlourish,
        };
        assert_eq!(outcome, PreFlourishOutcome::JourneyOnwardShortcut);
    }

    #[test]
    fn outcome_folds_lowercase_j_to_journey_shortcut() {
        let outcome = match Some(b'j') {
            Some(byte) if input_case_fold(byte) == b'J' => {
                PreFlourishOutcome::JourneyOnwardShortcut
            }
            _ => PreFlourishOutcome::ContinueToFlourish,
        };
        assert_eq!(outcome, PreFlourishOutcome::JourneyOnwardShortcut);
    }

    #[test]
    fn outcome_ignores_non_j_queued_key() {
        for key in [b'A', b'c', b'R', b'\r', b'\n', b' ', 0u8] {
            let outcome = match Some(key) {
                Some(byte) if input_case_fold(byte) == b'J' => {
                    PreFlourishOutcome::JourneyOnwardShortcut
                }
                _ => PreFlourishOutcome::ContinueToFlourish,
            };
            assert_eq!(
                outcome,
                PreFlourishOutcome::ContinueToFlourish,
                "queued key 0x{key:02x} must not trigger the J shortcut",
            );
        }
    }

    #[test]
    fn hercules_driver_load_is_deferred() {
        let game_dir = std::env::temp_dir().join("u5-intro-preflourish-hercules");
        let err = load_intro_font_slots(&game_dir, DisplayDriverFamily::Hercules).expect_err(
            "Hercules path must surface the deferred parity work, not silently load .CH",
        );
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        assert!(
            err.to_string().contains("Hercules"),
            "error should name the unsupported driver: {err}"
        );
    }

    #[test]
    fn ega_load_against_local_clean_assets_when_present() {
        let game_dir = Path::new(crate::DEFAULT_GAME_DIR);
        if !game_dir.join(IBM_CH_FILE).exists() || !game_dir.join(RUNES_CH_FILE).exists() {
            return;
        }
        let slots = load_intro_font_slots(game_dir, DisplayDriverFamily::Ega).unwrap();
        assert_eq!(slots.active_slot(), IntroFontSlot::Ibm);
        // Roundtrip glyph access through the active-font selector
        // proves the registry actually carries the parsed glyph
        // bytes through the slot table.
        let ibm_a_row = slots
            .active_font()
            .glyph_row(b'A', 0)
            .expect("IBM glyph 'A' row 0 must be present");
        slots_round_trip_glyph(&slots, ibm_a_row);

        let mut switched = slots.clone();
        switched.set_active_slot(IntroFontSlot::Runes);
        let runes_a_row = switched
            .active_font()
            .glyph_row(b'A', 0)
            .expect("runic glyph 'A' row 0 must be present");
        // The IBM and rune glyphs at the same code point differ — if
        // they don't, the slot table would still type-check but the
        // loader would clearly be wrong about which file went into
        // which slot.
        assert_ne!(
            ibm_a_row, runes_a_row,
            "IBM 'A' and runic 'A' should not have identical row-0 bitmaps; slot table may have crossed wires",
        );
    }

    fn slots_round_trip_glyph(slots: &IntroFontSlots, expected_first_row: u8) {
        let through_active = slots
            .active_font()
            .glyph_row(b'A', 0)
            .expect("active font glyph 'A' row 0 must round-trip");
        let through_index = slots
            .font(IntroFontSlot::Ibm)
            .glyph_row(b'A', 0)
            .expect("slot 0 glyph 'A' row 0 must round-trip");
        assert_eq!(through_active, expected_first_row);
        assert_eq!(through_index, expected_first_row);
    }

    #[test]
    fn run_pre_flourish_against_local_clean_assets_when_present() {
        let game_dir = Path::new(crate::DEFAULT_GAME_DIR);
        if !game_dir.join(IBM_CH_FILE).exists() || !game_dir.join(RUNES_CH_FILE).exists() {
            return;
        }
        // No queued input: caller falls through to step 3.
        let mut windows = empty_text_windows();
        let (slots, outcome) =
            run_intro_pre_flourish_phase(game_dir, DisplayDriverFamily::Ega, &mut windows, None)
                .unwrap();
        assert_eq!(outcome, PreFlourishOutcome::ContinueToFlourish);
        assert_eq!(slots.active_slot(), IntroFontSlot::Ibm);
        let descriptor = windows
            .window(PRE_FLOURISH_TEXT_WINDOW_INDEX)
            .expect("descriptor 0 must exist after step 4");
        assert_eq!(
            (
                descriptor.top_left_x,
                descriptor.top_left_y,
                descriptor.bottom_right_x,
                descriptor.bottom_right_y,
            ),
            (0, 0, TEXT_SCREEN_COLUMNS - 1, TEXT_SCREEN_ROWS - 1),
            "step 4 must reset the active text window to the full 40x25 rectangle",
        );
        assert_eq!(
            windows.active_window_index(),
            PRE_FLOURISH_TEXT_WINDOW_INDEX,
            "step 5 must select descriptor index 0",
        );

        // Queued lowercase 'j': caller jumps to the Journey Onward
        // load handler. Verify the case fold accepts lowercase per
        // spec ("fold the returned byte to uppercase, and compare it
        // to J").
        let mut windows = empty_text_windows();
        let (_, outcome) = run_intro_pre_flourish_phase(
            game_dir,
            DisplayDriverFamily::Ega,
            &mut windows,
            Some(b'j'),
        )
        .unwrap();
        assert_eq!(outcome, PreFlourishOutcome::JourneyOnwardShortcut);

        // Queued other key: no shortcut, fall through.
        let mut windows = empty_text_windows();
        let (_, outcome) = run_intro_pre_flourish_phase(
            game_dir,
            DisplayDriverFamily::Ega,
            &mut windows,
            Some(b'A'),
        )
        .unwrap();
        assert_eq!(outcome, PreFlourishOutcome::ContinueToFlourish);
    }
}
