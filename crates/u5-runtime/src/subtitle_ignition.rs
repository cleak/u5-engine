//! Intro start/menu subtitle ignition (`systems/intro.md` section 5).
//!
//! The carry-set half of display dispatch `0x69` is separate from the
//! ordinary title tick. It blanks the four staged title bands, walks one
//! fourteen-bit maximal Galois sequence twice, and uses `WD.BIT` to split the
//! restored pixels between a flame-field pass and a lettering pass. Every
//! nonzero sequence state is polled; only states mapping into the 288-by-49
//! footprint advance the per-pass publication countdown. A successfully
//! completed pass then applies its uncounted, unpublished `(0,0)` fixup.

use std::io;

use crate::{
    MonochromeBitmap, TITLE_TICK_FRAME_COUNT, TITLE_TICK_FRAME_HEIGHT, TITLE_TICK_FRAME_PIXELS,
    TITLE_TICK_FRAME_SET_BYTES, TITLE_TICK_FRAME_WIDTH, TITLE_TICK_SOURCE_WIDTH,
    TITLE_TICK_SOURCE_X, TitleTickFrameSet, title_tick_next_frame,
};

pub const SUBTITLE_IGNITION_MASK_WIDTH: usize = TITLE_TICK_SOURCE_WIDTH as usize;
pub const SUBTITLE_IGNITION_MASK_HEIGHT: usize = TITLE_TICK_FRAME_HEIGHT as usize;
pub const SUBTITLE_IGNITION_POSITION_COUNT: usize =
    SUBTITLE_IGNITION_MASK_WIDTH * SUBTITLE_IGNITION_MASK_HEIGHT;
/// Across both mask-selected passes every footprint position is restored once.
pub const SUBTITLE_IGNITION_RESTORATION_COUNT: usize = SUBTITLE_IGNITION_POSITION_COUNT;
pub const SUBTITLE_IGNITION_LFSR_BITS: u32 = 14;
pub const SUBTITLE_IGNITION_LFSR_STATE_COUNT: usize = (1 << SUBTITLE_IGNITION_LFSR_BITS) - 1;
/// Published fourteen-bit Galois feedback tap (`systems/intro.md §5`,
/// clean commit `36780cb`). The current state is mapped before this
/// right-shift/conditional-XOR update.
pub const SUBTITLE_IGNITION_LFSR_TAP: u16 = 0x3500;
pub const SUBTITLE_IGNITION_IN_BOUNDS_NONZERO_STATES_PER_PASS: usize =
    SUBTITLE_IGNITION_POSITION_COUNT - 1;
pub const SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_NORMAL: usize = 128;
pub const SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_SLOW: usize = 256;
pub const SUBTITLE_IGNITION_NORMAL_PUBLISHES_PER_PASS: usize = 110;
pub const SUBTITLE_IGNITION_SLOW_PUBLISHES_PER_PASS: usize = 55;
pub const SUBTITLE_IGNITION_UNPUBLISHED_COUNTED_TAIL_PER_PASS: usize = 31;
pub const SUBTITLE_IGNITION_UNPUBLISHED_TAIL_WITH_FIXUP_PER_PASS: usize = 32;
pub const SUBTITLE_IGNITION_CALIBRATION_THRESHOLD: u16 = 250;
pub const SUBTITLE_IGNITION_DRIVER_STATE_SEED: u16 = 0x7664;
pub const SUBTITLE_IGNITION_DRIVER_STATE_ADD: u16 = 0x9248;
pub const SUBTITLE_IGNITION_DRIVER_STATE_FINAL_ADD: u16 = 0x0011;
pub const SUBTITLE_IGNITION_SPEAKER_PITCHES_PER_BURST: usize = 25;
/// `intro.md §5`: the requested frequency of one burst pitch is
/// `100 + (pitch_state modulo 1401)`, i.e. 100 through 1500 Hz.
pub const SUBTITLE_IGNITION_PITCH_BASE_HZ: u16 = 100;
pub const SUBTITLE_IGNITION_PITCH_MODULUS: u16 = 1401;
pub const SUBTITLE_IGNITION_SPEAKER_WAIT_UNITS: u8 = 45;
pub const SUBTITLE_IGNITION_SILENT_WAIT_UNITS: u8 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleIgnitionPass {
    FlameField,
    Lettering,
}

impl SubtitleIgnitionPass {
    const ALL: [Self; 2] = [Self::FlameField, Self::Lettering];

    const fn accepts(self, mask_bit: u8) -> bool {
        match self {
            Self::FlameField => mask_bit == 0,
            Self::Lettering => mask_bit == 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleIgnitionCadence {
    Normal128,
    Slow256,
}

impl SubtitleIgnitionCadence {
    pub const fn from_boot_calibration(calibration: u16) -> Self {
        if calibration < SUBTITLE_IGNITION_CALIBRATION_THRESHOLD {
            Self::Slow256
        } else {
            Self::Normal128
        }
    }

    pub const fn positions_per_publish(self) -> usize {
        match self {
            Self::Normal128 => SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_NORMAL,
            Self::Slow256 => SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_SLOW,
        }
    }

    pub const fn publishes_per_pass(self) -> usize {
        match self {
            Self::Normal128 => SUBTITLE_IGNITION_NORMAL_PUBLISHES_PER_PASS,
            Self::Slow256 => SUBTITLE_IGNITION_SLOW_PUBLISHES_PER_PASS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtitleIgnitionRestore {
    pub pass: SubtitleIgnitionPass,
    pub x: u16,
    pub y: u16,
    /// True only for the uncounted, unpaced and unpolled `(0,0)` fixup.
    pub corner_fixup: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubtitleIgnitionPublish {
    /// Free-running counter value drawn before the counter advances.
    pub frame: u8,
    pub pass: SubtitleIgnitionPass,
    /// Completed in-bounds nonzero states in this pass.
    pub in_bounds_positions_in_pass: usize,
    /// Mask-selected footprint positions restored across both passes so far.
    pub restored_positions: usize,
    /// Global keyboard-status poll performed after this publication's state.
    pub keyboard_poll_after_state: usize,
    /// Current fourteen-bit state whose processing reached this boundary.
    pub lfsr_state: u16,
    /// Current gate tested before the iteration's post-poll advance.
    pub gate_state: u16,
    pub speaker_burst: bool,
    /// `intro.md §5`: the twenty-five successive frequencies this publication
    /// programs, in order. Empty on a silent publication.
    pub burst_pitches: Vec<u16>,
    pub wait_units: u8,
    /// Partially restored full-width `320 x 49` band.
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubtitleIgnitionPlayback {
    pub cadence: SubtitleIgnitionCadence,
    pub publishes: Vec<SubtitleIgnitionPublish>,
    pub final_frame: u8,
    pub restored_positions: usize,
    pub keyboard_polls: usize,
    pub corner_fixups: usize,
    pub speaker_bursts: usize,
    pub speaker_pitch_steps: usize,
    pub final_gate_state: u16,
    pub final_pitch_state: u16,
    /// Counted in-bounds states left after the last publication of each pass.
    pub unpublished_counted_tail_per_pass: [usize; 2],
}

fn validate_mask(mask: &MonochromeBitmap) -> io::Result<()> {
    if (mask.width, mask.height) != (SUBTITLE_IGNITION_MASK_WIDTH, SUBTITLE_IGNITION_MASK_HEIGHT) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "WD.BIT ignition mask is {}x{}, expected {}x{}",
                mask.width,
                mask.height,
                SUBTITLE_IGNITION_MASK_WIDTH,
                SUBTITLE_IGNITION_MASK_HEIGHT
            ),
        ));
    }
    if mask.pixels.len() != SUBTITLE_IGNITION_POSITION_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "WD.BIT ignition mask has {} pixels, expected {}",
                mask.pixels.len(),
                SUBTITLE_IGNITION_POSITION_COUNT
            ),
        ));
    }
    if let Some((index, value)) = mask
        .pixels
        .iter()
        .copied()
        .enumerate()
        .find(|(_, value)| *value > 1)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("WD.BIT ignition mask pixel {index} is {value}, expected binary 0 or 1"),
        ));
    }
    Ok(())
}

const fn advance_subtitle_ignition_lfsr(state: u16) -> u16 {
    let mut next = state >> 1;
    if state & 1 != 0 {
        next ^= SUBTITLE_IGNITION_LFSR_TAP;
    }
    next & ((1 << SUBTITLE_IGNITION_LFSR_BITS) - 1) as u16
}

/// `intro.md §5`: map one advanced pitch state onto its requested frequency.
pub const fn subtitle_ignition_burst_pitch(pitch_state: u16) -> u16 {
    SUBTITLE_IGNITION_PITCH_BASE_HZ + pitch_state % SUBTITLE_IGNITION_PITCH_MODULUS
}

pub const fn advance_subtitle_ignition_driver_state(state: u16) -> u16 {
    let mixed = state
        .wrapping_add(SUBTITLE_IGNITION_DRIVER_STATE_ADD)
        .rotate_right(3)
        ^ SUBTITLE_IGNITION_DRIVER_STATE_ADD;
    mixed.wrapping_add(SUBTITLE_IGNITION_DRIVER_STATE_FINAL_ADD)
}

fn for_each_nonzero_lfsr_state(mut visit: impl FnMut(u16)) {
    let mut state = 1u16;
    loop {
        visit(state);
        state = advance_subtitle_ignition_lfsr(state);
        if state == 1 {
            break;
        }
    }
}

fn state_position(state: u16) -> Option<(usize, usize, usize)> {
    let state = usize::from(state);
    let y = state / SUBTITLE_IGNITION_MASK_WIDTH;
    (y < SUBTITLE_IGNITION_MASK_HEIGHT).then(|| {
        let x = state % SUBTITLE_IGNITION_MASK_WIDTH;
        (state, x, y)
    })
}

fn restore_position(staged: &mut [u8], frames: &TitleTickFrameSet, x: usize, y: usize) {
    let x = x + TITLE_TICK_SOURCE_X as usize;
    let cell = y * TITLE_TICK_FRAME_WIDTH as usize + x;
    for source_frame in 0..TITLE_TICK_FRAME_COUNT {
        let source = frames.frame_pixels(source_frame)[cell];
        let destination = usize::from(source_frame) * TITLE_TICK_FRAME_PIXELS + cell;
        staged[destination] = source;
    }
}

/// Build the two-pass mask-selected restoration plan, including the explicit
/// origin fixup in whichever pass owns that mask bit. Every footprint position
/// therefore appears exactly once.
pub fn subtitle_ignition_restoration_plan(
    mask: &MonochromeBitmap,
) -> io::Result<Vec<SubtitleIgnitionRestore>> {
    validate_mask(mask)?;
    let mut restores = Vec::with_capacity(SUBTITLE_IGNITION_RESTORATION_COUNT);
    for pass in SubtitleIgnitionPass::ALL {
        for_each_nonzero_lfsr_state(|state| {
            let Some((index, x, y)) = state_position(state) else {
                return;
            };
            if pass.accepts(mask.pixels[index]) {
                restores.push(SubtitleIgnitionRestore {
                    pass,
                    x: x as u16,
                    y: y as u16,
                    corner_fixup: false,
                });
            }
        });
        if pass.accepts(mask.pixels[0]) {
            restores.push(SubtitleIgnitionRestore {
                pass,
                x: 0,
                y: 0,
                corner_fixup: true,
            });
        }
    }
    debug_assert_eq!(restores.len(), SUBTITLE_IGNITION_RESTORATION_COUNT);
    Ok(restores)
}

/// Expand the normal (calibration >= 250) uninterrupted transition.
pub fn build_subtitle_ignition_playback(
    frames: &TitleTickFrameSet,
    mask: &MonochromeBitmap,
    starting_frame: u8,
) -> io::Result<SubtitleIgnitionPlayback> {
    build_subtitle_ignition_playback_with_driver_state(
        frames,
        mask,
        starting_frame,
        SUBTITLE_IGNITION_CALIBRATION_THRESHOLD,
        SUBTITLE_IGNITION_DRIVER_STATE_SEED,
        SUBTITLE_IGNITION_DRIVER_STATE_SEED,
    )
}

/// Expand one uninterrupted transition with explicit persistent driver state.
/// This keeps the calibration branch and speaker/pitch recurrences testable
/// without requiring a physical PC-speaker backend.
pub fn build_subtitle_ignition_playback_with_driver_state(
    frames: &TitleTickFrameSet,
    mask: &MonochromeBitmap,
    starting_frame: u8,
    boot_calibration: u16,
    mut gate_state: u16,
    mut pitch_state: u16,
) -> io::Result<SubtitleIgnitionPlayback> {
    validate_mask(mask)?;
    let cadence = SubtitleIgnitionCadence::from_boot_calibration(boot_calibration);
    let positions_per_publish = cadence.positions_per_publish();
    let mut staged = vec![0u8; TITLE_TICK_FRAME_SET_BYTES];
    let mut frame = starting_frame % TITLE_TICK_FRAME_COUNT;
    let mut restored_positions = 0usize;
    let mut keyboard_polls = 0usize;
    let mut corner_fixups = 0usize;
    let mut speaker_bursts = 0usize;
    let mut publishes = Vec::with_capacity(cadence.publishes_per_pass() * 2);
    let mut unpublished_counted_tail_per_pass = [0usize; 2];

    for (pass_index, pass) in SubtitleIgnitionPass::ALL.into_iter().enumerate() {
        let mut in_bounds_positions = 0usize;
        let mut since_publish = 0usize;
        let mut publication_in_pass = 0usize;

        for_each_nonzero_lfsr_state(|state| {
            if let Some((index, x, y)) = state_position(state) {
                in_bounds_positions += 1;
                since_publish += 1;
                if pass.accepts(mask.pixels[index]) {
                    restore_position(&mut staged, frames, x, y);
                    restored_positions += 1;
                }

                if since_publish == positions_per_publish {
                    publication_in_pass += 1;
                    let threshold = 400usize - 3 * publication_in_pass;
                    let speaker_burst = usize::from(gate_state & 0x01ff) < threshold;
                    let mut burst_pitches = Vec::new();
                    let wait_units = if speaker_burst {
                        speaker_bursts += 1;
                        burst_pitches.reserve_exact(SUBTITLE_IGNITION_SPEAKER_PITCHES_PER_BURST);
                        for _ in 0..SUBTITLE_IGNITION_SPEAKER_PITCHES_PER_BURST {
                            pitch_state = advance_subtitle_ignition_driver_state(pitch_state);
                            burst_pitches.push(subtitle_ignition_burst_pitch(pitch_state));
                        }
                        SUBTITLE_IGNITION_SPEAKER_WAIT_UNITS
                    } else {
                        SUBTITLE_IGNITION_SILENT_WAIT_UNITS
                    };
                    let start = usize::from(frame) * TITLE_TICK_FRAME_PIXELS;
                    publishes.push(SubtitleIgnitionPublish {
                        frame,
                        pass,
                        in_bounds_positions_in_pass: in_bounds_positions,
                        restored_positions,
                        keyboard_poll_after_state: keyboard_polls + 1,
                        lfsr_state: state,
                        gate_state,
                        speaker_burst,
                        burst_pitches,
                        wait_units,
                        pixels: staged[start..start + TITLE_TICK_FRAME_PIXELS].to_vec(),
                    });
                    frame = title_tick_next_frame(frame);
                    since_publish = 0;
                }
            }

            keyboard_polls += 1;
            gate_state = advance_subtitle_ignition_driver_state(gate_state);
        });

        unpublished_counted_tail_per_pass[pass_index] = since_publish;
        corner_fixups += 1;
        if pass.accepts(mask.pixels[0]) {
            restore_position(&mut staged, frames, 0, 0);
            restored_positions += 1;
        }
    }

    Ok(SubtitleIgnitionPlayback {
        cadence,
        publishes,
        final_frame: frame,
        restored_positions,
        keyboard_polls,
        corner_fixups,
        speaker_bursts,
        speaker_pitch_steps: speaker_bursts * SUBTITLE_IGNITION_SPEAKER_PITCHES_PER_BURST,
        final_gate_state: gate_state,
        final_pitch_state: pitch_state,
        unpublished_counted_tail_per_pass,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask_with_lettering(predicate: impl Fn(usize, usize) -> bool) -> MonochromeBitmap {
        let mut pixels = vec![0; SUBTITLE_IGNITION_POSITION_COUNT];
        for y in 0..SUBTITLE_IGNITION_MASK_HEIGHT {
            for x in 0..SUBTITLE_IGNITION_MASK_WIDTH {
                pixels[y * SUBTITLE_IGNITION_MASK_WIDTH + x] = u8::from(predicate(x, y));
            }
        }
        MonochromeBitmap {
            width: SUBTITLE_IGNITION_MASK_WIDTH,
            height: SUBTITLE_IGNITION_MASK_HEIGHT,
            pixels,
        }
    }

    fn nonzero_frame_set() -> TitleTickFrameSet {
        let mut pixels = vec![0; TITLE_TICK_FRAME_SET_BYTES];
        for frame in 0..TITLE_TICK_FRAME_COUNT {
            let start = usize::from(frame) * TITLE_TICK_FRAME_PIXELS;
            pixels[start..start + TITLE_TICK_FRAME_PIXELS].fill(frame + 1);
        }
        TitleTickFrameSet::from_palette_indices(pixels, "subtitle ignition test frames").unwrap()
    }

    #[test]
    fn fourteen_bit_walk_visits_every_nonzero_state_once() {
        let mut seen = vec![false; 1 << SUBTITLE_IGNITION_LFSR_BITS];
        let mut count = 0;
        for_each_nonzero_lfsr_state(|state| {
            assert_ne!(state, 0);
            assert!(!seen[usize::from(state)], "state {state:#06x} repeated");
            seen[usize::from(state)] = true;
            count += 1;
        });
        assert_eq!(count, SUBTITLE_IGNITION_LFSR_STATE_COUNT);
        assert!(seen[1..].iter().all(|visited| *visited));
    }

    #[test]
    fn fourteen_bit_walk_matches_published_first_sixteen_state_vector() {
        let expected = [
            (0x0001, 1, 0),
            (0x3500, 32, 47),
            (0x1a80, 160, 23),
            (0x0d40, 224, 11),
            (0x06a0, 256, 5),
            (0x0350, 272, 2),
            (0x01a8, 136, 1),
            (0x00d4, 212, 0),
            (0x006a, 106, 0),
            (0x0035, 53, 0),
            (0x351a, 58, 47),
            (0x1a8d, 173, 23),
            (0x3846, 6, 50),
            (0x1c23, 3, 25),
            (0x3b11, 145, 52),
            (0x2888, 8, 36),
        ];
        let mut state = 1u16;
        for &(expected_state, expected_x, expected_y) in &expected {
            assert_eq!(state, expected_state);
            assert_eq!(state % SUBTITLE_IGNITION_MASK_WIDTH as u16, expected_x);
            assert_eq!(state / SUBTITLE_IGNITION_MASK_WIDTH as u16, expected_y);
            state = advance_subtitle_ignition_lfsr(state);
        }
    }

    #[test]
    fn restoration_plan_splits_mask_positions_and_applies_one_origin_fixup() {
        let mask = mask_with_lettering(|x, y| (x + y) % 3 == 0);
        let plan = subtitle_ignition_restoration_plan(&mask).unwrap();
        assert_eq!(plan.len(), SUBTITLE_IGNITION_RESTORATION_COUNT);
        assert_eq!(
            plan.iter().filter(|restore| restore.corner_fixup).count(),
            1
        );

        let first_lettering = plan
            .iter()
            .position(|restore| restore.pass == SubtitleIgnitionPass::Lettering)
            .expect("synthetic mask has lettering positions");
        assert!(
            plan[..first_lettering]
                .iter()
                .all(|restore| restore.pass == SubtitleIgnitionPass::FlameField)
        );
        assert!(
            plan[first_lettering..]
                .iter()
                .all(|restore| restore.pass == SubtitleIgnitionPass::Lettering)
        );

        let mut seen = vec![false; SUBTITLE_IGNITION_POSITION_COUNT];
        for restore in &plan {
            let index =
                usize::from(restore.y) * SUBTITLE_IGNITION_MASK_WIDTH + usize::from(restore.x);
            assert!(!seen[index], "position {index} restored twice");
            seen[index] = true;
            assert_eq!(
                restore.pass,
                if mask.pixels[index] == 0 {
                    SubtitleIgnitionPass::FlameField
                } else {
                    SubtitleIgnitionPass::Lettering
                }
            );
        }
        assert!(seen.iter().all(|visited| *visited));
    }

    #[test]
    fn normal_playback_resets_each_pass_countdown_and_keeps_tails_unpublished() {
        let mask = mask_with_lettering(|_, _| false);
        let playback = build_subtitle_ignition_playback(&nonzero_frame_set(), &mask, 3).unwrap();
        assert_eq!(playback.cadence, SubtitleIgnitionCadence::Normal128);
        assert_eq!(playback.publishes.len(), 220);
        assert_eq!(
            playback.restored_positions,
            SUBTITLE_IGNITION_POSITION_COUNT
        );
        assert_eq!(
            playback.keyboard_polls,
            2 * SUBTITLE_IGNITION_LFSR_STATE_COUNT
        );
        assert_eq!(playback.corner_fixups, 2);
        assert_eq!(
            playback.unpublished_counted_tail_per_pass,
            [SUBTITLE_IGNITION_UNPUBLISHED_COUNTED_TAIL_PER_PASS; 2]
        );
        assert_eq!(playback.final_frame, 3);

        let last_flame = &playback.publishes[SUBTITLE_IGNITION_NORMAL_PUBLISHES_PER_PASS - 1];
        let first_lettering = &playback.publishes[SUBTITLE_IGNITION_NORMAL_PUBLISHES_PER_PASS];
        assert_eq!(last_flame.pass, SubtitleIgnitionPass::FlameField);
        assert_eq!(
            last_flame.in_bounds_positions_in_pass,
            SUBTITLE_IGNITION_NORMAL_PUBLISHES_PER_PASS
                * SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_NORMAL
        );
        assert_eq!(
            last_flame
                .pixels
                .iter()
                .filter(|value| **value != 0)
                .count(),
            SUBTITLE_IGNITION_NORMAL_PUBLISHES_PER_PASS
                * SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_NORMAL
        );
        assert_eq!(first_lettering.pass, SubtitleIgnitionPass::Lettering);
        assert_eq!(
            first_lettering
                .pixels
                .iter()
                .filter(|value| **value != 0)
                .count(),
            SUBTITLE_IGNITION_POSITION_COUNT,
            "pass one's 31-state tail plus corner fixup first appear in pass two"
        );
    }

    #[test]
    fn slow_calibration_uses_256_positions_and_leaves_counter_at_two() {
        let mask = mask_with_lettering(|x, y| (x + y) % 5 == 0);
        let playback = build_subtitle_ignition_playback_with_driver_state(
            &nonzero_frame_set(),
            &mask,
            0,
            SUBTITLE_IGNITION_CALIBRATION_THRESHOLD - 1,
            SUBTITLE_IGNITION_DRIVER_STATE_SEED,
            SUBTITLE_IGNITION_DRIVER_STATE_SEED,
        )
        .unwrap();
        assert_eq!(playback.cadence, SubtitleIgnitionCadence::Slow256);
        assert_eq!(playback.publishes.len(), 110);
        assert_eq!(playback.final_frame, 2);
        assert_eq!(
            playback.unpublished_counted_tail_per_pass,
            [SUBTITLE_IGNITION_UNPUBLISHED_COUNTED_TAIL_PER_PASS; 2]
        );
    }

    #[test]
    fn publications_draw_before_advance_and_restore_all_four_bands_together() {
        let mask = mask_with_lettering(|_, _| false);
        let playback = build_subtitle_ignition_playback(&nonzero_frame_set(), &mask, 2).unwrap();
        let first = &playback.publishes[0];
        assert_eq!(first.frame, 2);
        assert_eq!(
            first.in_bounds_positions_in_pass,
            SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_NORMAL
        );
        assert_eq!(
            first.restored_positions,
            SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_NORMAL
        );
        assert_eq!(
            first.pixels.iter().filter(|value| **value == 3).count(),
            SUBTITLE_IGNITION_POSITIONS_PER_PUBLISH_NORMAL
        );
        assert!(first.pixels.iter().all(|value| matches!(*value, 0 | 3)));
    }

    #[test]
    fn gate_state_selects_wait_branches_and_pitch_step_counts() {
        let mask = mask_with_lettering(|x, y| (x + y) % 2 == 0);
        let normal = build_subtitle_ignition_playback(&nonzero_frame_set(), &mask, 0).unwrap();
        assert_eq!(
            normal.speaker_bursts,
            normal
                .publishes
                .iter()
                .filter(|publish| publish.speaker_burst)
                .count()
        );
        assert_eq!(
            normal
                .publishes
                .iter()
                .filter(|publish| {
                    publish.pass == SubtitleIgnitionPass::FlameField && publish.speaker_burst
                })
                .count(),
            48
        );
        assert_eq!(
            normal
                .publishes
                .iter()
                .filter(|publish| {
                    publish.pass == SubtitleIgnitionPass::Lettering && publish.speaker_burst
                })
                .count(),
            53
        );
        let first = &normal.publishes[0];
        assert_eq!(first.keyboard_poll_after_state, 145);
        assert_eq!(first.lfsr_state, 0x0654);
        assert_eq!(first.gate_state, 0x15aa);
        assert!(!first.speaker_burst);
        let second = &normal.publishes[1];
        assert_eq!(second.keyboard_poll_after_state, 296);
        assert_eq!(second.lfsr_state, 0x2562);
        assert_eq!(second.gate_state, 0x7283);
        assert!(second.speaker_burst);
        assert_eq!(
            normal.speaker_pitch_steps,
            normal.speaker_bursts * SUBTITLE_IGNITION_SPEAKER_PITCHES_PER_BURST
        );
        assert!(normal.publishes.iter().all(|publish| {
            publish.wait_units
                == if publish.speaker_burst {
                    SUBTITLE_IGNITION_SPEAKER_WAIT_UNITS
                } else {
                    SUBTITLE_IGNITION_SILENT_WAIT_UNITS
                }
        }));

        let slow = build_subtitle_ignition_playback_with_driver_state(
            &nonzero_frame_set(),
            &mask,
            0,
            SUBTITLE_IGNITION_CALIBRATION_THRESHOLD - 1,
            SUBTITLE_IGNITION_DRIVER_STATE_SEED,
            SUBTITLE_IGNITION_DRIVER_STATE_SEED,
        )
        .unwrap();
        assert_eq!(
            slow.speaker_bursts,
            slow.publishes
                .iter()
                .filter(|publish| publish.speaker_burst)
                .count()
        );
        assert_eq!(
            slow.publishes
                .iter()
                .filter(|publish| {
                    publish.pass == SubtitleIgnitionPass::FlameField && publish.speaker_burst
                })
                .count(),
            35
        );
        assert_eq!(
            slow.publishes
                .iter()
                .filter(|publish| {
                    publish.pass == SubtitleIgnitionPass::Lettering && publish.speaker_burst
                })
                .count(),
            33
        );
        assert_eq!(slow.publishes[0].keyboard_poll_after_state, 296);
        assert_eq!(slow.publishes[0].lfsr_state, 0x2562);
        assert_eq!(slow.publishes[0].gate_state, 0x7283);
        assert!(slow.publishes[0].speaker_burst);
        assert!(slow.publishes.iter().all(|publish| {
            publish.wait_units
                == if publish.speaker_burst {
                    SUBTITLE_IGNITION_SPEAKER_WAIT_UNITS
                } else {
                    SUBTITLE_IGNITION_SILENT_WAIT_UNITS
                }
        }));
    }

    #[test]
    fn ignition_rejects_wrong_geometry_and_nonbinary_pixels() {
        let frames = nonzero_frame_set();
        let wrong = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![0],
        };
        assert!(build_subtitle_ignition_playback(&frames, &wrong, 0).is_err());

        let mut nonbinary = mask_with_lettering(|_, _| false);
        nonbinary.pixels[17] = 2;
        assert!(build_subtitle_ignition_playback(&frames, &nonbinary, 0).is_err());
    }
}
