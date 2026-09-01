//! Renders a [`SpeakerProgram`] to mono PCM samples.
//!
//! `systems/audio.md §1` permits this: "A modern frontend may synthesize
//! equivalent waveforms instead of emulating timer ports, provided it preserves
//! the stated trigger, cancellation, ordering, blocking, mute, and random-state
//! boundaries." The boundaries live in [`crate::audio`]; this module only turns
//! the resulting operation list into audio.
//!
//! The model is deliberately literal about the hardware it is imitating. The
//! speaker is one bit (`audio.md §2`), so every sample is a square-wave value,
//! never a filtered or shaped instrument tone:
//!
//! - a [`SpeakerOp::Tone`] emits a square wave at the frequency its *divisor*
//!   actually produces, not the frequency that was requested, because
//!   `audio.md §5.1` truncates the divisor;
//! - a [`SpeakerOp::Envelope`] is **not** a pin toggle. `cleak/u5-spec#146`
//!   corrected that: the loop programs the timer once with an inaudible
//!   ~19,886 Hz carrier and gates it on and off, so the audible waveform is the
//!   gate pattern, completing exactly `period / 65536` cycles per iteration.
//!   The renderer evaluates that gate at the envelope's own iteration rate and
//!   box-filters it down to the output rate, which is what makes the moving
//!   duty cycle read as a timbre sweep rather than as aliasing noise. The
//!   carrier itself sits above the top of the audible band and above what
//!   44.1 kHz can represent without folding, so it is not synthesised;
//! - a [`SpeakerOp::Stop`] silences immediately.
//!
//! Phase is continuous across consecutive tones, so a glissando reads as one
//! swept voice rather than as a series of retriggered blips.

use crate::audio::{
    ENVELOPE_MODULUS, EnvelopeSegment, SpeakerOp, SpeakerProgram, frequency_for_divisor,
};

/// Output sample rate.
pub const SAMPLE_RATE: u32 = 44_100;

/// Peak amplitude of the one-bit speaker signal.
///
/// The historical speaker is a full-swing square wave. This is scaled well
/// below 1.0 because several published effects retune hundreds of times per
/// second and a full-scale square wave at those rates is painful on modern
/// hardware.
pub const PEAK_AMPLITUDE: f32 = 0.32;

/// Cutoff of the DC blocker, in hertz.
///
/// A real speaker cone is AC-coupled and cannot hold a static displacement.
/// This matters for `audio.md §5.4`: the software envelope sweeps its duty
/// cycle from a few percent to fifty, and a narrow-duty square carries a large
/// DC component. Without blocking it, the published timbre sweep renders as a
/// thump plus a quiet pulse train instead of as the brightening whoosh the
/// moving comparison value describes. The cutoff sits below the 19 Hz floor of
/// the `§8.4` flash bands so no published tone is attenuated.
const DC_BLOCK_CUTOFF_HZ: f32 = 10.0;

/// Length of the anti-click ramp applied at the very start and end of a
/// rendered program, in samples.
///
/// This is applied once per program, never per operation: ramping every tone
/// would smooth away the hard retunes that give the glissando and rumble
/// families their character.
const EDGE_RAMP_SAMPLES: usize = 64;

/// A rendered program.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedSpeaker {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl RenderedSpeaker {
    pub fn duration_secs(&self) -> f64 {
        self.samples.len() as f64 / f64::from(self.sample_rate)
    }

    /// Whether the render is audibly non-silent.
    pub fn is_silent(&self) -> bool {
        self.samples.iter().all(|sample| sample.abs() < 1e-6)
    }

    /// Peak absolute amplitude, for smoke checks.
    pub fn peak(&self) -> f32 {
        self.samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
    }
}

fn samples_for_nanos(nanos: u64) -> usize {
    (nanos as f64 / 1.0e9 * f64::from(SAMPLE_RATE)).round() as usize
}

/// Closed-form gate state of an [`EnvelopeSegment`] at iteration `index`.
///
/// `audio.md §5.4` gives the recurrence; because both the phase and the
/// comparison value are affine in the iteration index modulo 65536, the state
/// at any iteration is computable directly. That is what lets the renderer
/// box-filter the bitstream without materializing it.
fn envelope_pin_at(segment: &EnvelopeSegment, index: u64) -> bool {
    let modulus = u64::from(ENVELOPE_MODULUS);
    let phase = (index + 1) % modulus * u64::from(segment.period) % modulus;
    let delta = segment.delta.rem_euclid(ENVELOPE_MODULUS as i32) as u64;
    let comparison = (u64::from(segment.initial_comparison) + index % modulus * delta) % modulus;
    phase >= comparison
}

/// Render one program to mono PCM.
pub fn render_program(program: &SpeakerProgram, audible: bool) -> RenderedSpeaker {
    let total = samples_for_nanos(program.total_nanos(audible));
    let mut samples: Vec<f32> = Vec::with_capacity(total + 1);
    // Continuous square-wave phase in turns, so consecutive tones do not click.
    let mut phase = 0.0_f64;

    for op in &program.ops {
        match op {
            SpeakerOp::Tone { divisor, nanos, .. } => {
                let count = samples_for_nanos(*nanos);
                let frequency = f64::from(frequency_for_divisor(*divisor));
                let increment = frequency / f64::from(SAMPLE_RATE);
                for _ in 0..count {
                    phase = (phase + increment).fract();
                    samples.push(if phase < 0.5 {
                        PEAK_AMPLITUDE
                    } else {
                        -PEAK_AMPLITUDE
                    });
                }
            }
            SpeakerOp::Silence { nanos } => {
                let count = samples_for_nanos(*nanos);
                samples.extend(std::iter::repeat_n(0.0, count));
            }
            SpeakerOp::Envelope(segment) => {
                let count = samples_for_nanos(segment.total_nanos(audible));
                if count == 0 {
                    continue;
                }
                let iterations_per_sample = segment.iterations as f64 / count as f64;
                for index in 0..count {
                    // Box-filter the pin bitstream over this sample's window.
                    let start = (index as f64 * iterations_per_sample).floor() as u64;
                    let end = (((index + 1) as f64 * iterations_per_sample).ceil() as u64)
                        .max(start + 1)
                        .min(u64::from(segment.iterations));
                    let mut high = 0u32;
                    let mut total_bits = 0u32;
                    for iteration in start..end {
                        if envelope_pin_at(segment, iteration) {
                            high += 1;
                        }
                        total_bits += 1;
                    }
                    let mean = if total_bits == 0 {
                        0.0
                    } else {
                        f64::from(high) / f64::from(total_bits)
                    };
                    // Map the duty-cycle mean back onto the one-bit swing.
                    samples.push(((mean * 2.0 - 1.0) as f32) * PEAK_AMPLITUDE);
                }
            }
            SpeakerOp::Stop => {}
        }
    }

    apply_dc_block(&mut samples);
    apply_edge_ramp(&mut samples);
    RenderedSpeaker {
        samples,
        sample_rate: SAMPLE_RATE,
    }
}

/// One-pole high-pass, applied once across the whole program.
fn apply_dc_block(samples: &mut [f32]) {
    let coefficient = 1.0 - std::f32::consts::TAU * DC_BLOCK_CUTOFF_HZ / SAMPLE_RATE as f32;
    let mut previous_input = 0.0_f32;
    let mut previous_output = 0.0_f32;
    for sample in samples.iter_mut() {
        let input = *sample;
        let output = input - previous_input + coefficient * previous_output;
        previous_input = input;
        previous_output = output;
        *sample = output;
    }
}

fn apply_edge_ramp(samples: &mut [f32]) {
    let ramp = EDGE_RAMP_SAMPLES.min(samples.len() / 2);
    if ramp == 0 {
        return;
    }
    let len = samples.len();
    for index in 0..ramp {
        let gain = index as f32 / ramp as f32;
        samples[index] *= gain;
        samples[len - 1 - index] *= gain;
    }
}

/// Encode a render as a 16-bit mono WAV file.
///
/// This exists so audio can be verified offline — durations, peaks, and the
/// actual waveform — without a graphical shell or an audio device.
pub fn wav_bytes(render: &RenderedSpeaker) -> Vec<u8> {
    let data_len = render.samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    let byte_rate = render.sample_rate * 2;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&render.sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in &render.samples {
        let clamped = sample.clamp(-1.0, 1.0);
        out.extend_from_slice(&((clamped * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{self, RumbleJitter, SoundEffect};

    /// `audio.md §8.6.1`: "the speaker's square wave runs continuously from the
    /// first click to the end of the dissolve, and is retuned to a fresh
    /// pseudorandom frequency every second visited pixel. The 'hiss' is neither
    /// a train of discrete clicks nor the raw copy rate."
    ///
    /// The renderer already carries square-wave phase across consecutive tones,
    /// which is what makes a retune a retune rather than a restart. That was
    /// inert while every click was lowered as its own `[Tone, Stop]` program;
    /// with the per-click stop withdrawn it is load-bearing.
    ///
    /// The window is taken from mid-dissolve deliberately. §8.6.1: early in the
    /// run "the programmed half-period (about 4 to 5 ms at 100 to 120 Hz) is
    /// much longer than the retune interval", so an opening slice legitimately
    /// renders as a fraction of one half-cycle and shows no swing at all. "By
    /// mid-dissolve the half-period ... is shorter than the retune interval and
    /// essentially every write lands" — that is the part a continuous carrier
    /// has to reproduce.
    #[test]
    fn a_dissolve_run_renders_as_one_continuous_carrier() {
        let mut tone = audio::DissolveToneState::on_driver_load();
        let pitches: Vec<u16> = (0..8_192).map(|_| tone.next_click_hz()).collect();
        let mid_run = &pitches[8_000..];
        // Not every mid-run pitch is high — §8.6.1 is explicit that low pops
        // scatter through "right to the end" — but the mean has climbed far
        // above the opening cluster near 260 Hz.
        let mean: u32 = mid_run.iter().map(|hz| u32::from(*hz)).sum::<u32>() / mid_run.len() as u32;
        assert!(mean > 1_000, "mid-dissolve mean climbed to {mean} Hz");

        let rendered = render_program(&audio::dissolve_click_run(mid_run), true);
        assert!(rendered.samples.len() > mid_run.len());

        // The carrier oscillates throughout: split the run into windows and
        // require every one to swing both ways. A per-click stop, or any
        // interior gap, leaves a flat window. (Asserting on exact zeros would
        // not work here — apply_dc_block shifts the rails off zero.)
        let windows = 8;
        let width = rendered.samples.len() / windows;
        for window in 0..windows {
            let slice = &rendered.samples[window * width..(window + 1) * width];
            assert!(
                slice.iter().any(|s| *s > 0.0) && slice.iter().any(|s| *s < 0.0),
                "window {window} is flat: the carrier stopped mid-dissolve",
            );
        }
    }

    fn render(effect: SoundEffect) -> RenderedSpeaker {
        render_program(&effect.program(&mut RumbleJitter::new()), true)
    }

    #[test]
    fn blocked_step_renders_the_published_duration_and_pitch() {
        // audio.md §7.4: 165 Hz for 200 calibrated units.
        let rendered = render(SoundEffect::BlockedStep);
        let expected = audio::tone_nanos(200) as f64 / 1.0e9;
        assert!(
            (rendered.duration_secs() - expected).abs() < 1e-3,
            "expected about {expected}s, got {}s",
            rendered.duration_secs(),
        );
        assert!(!rendered.is_silent());

        // Count zero crossings to recover the played frequency. The divisor
        // truncation means the speaker plays slightly off the request.
        let played = f64::from(audio::frequency_for_divisor(audio::pit_divisor(165)));
        let crossings = rendered
            .samples
            .windows(2)
            .filter(|pair| pair[0].signum() != pair[1].signum())
            .count();
        let measured = crossings as f64 / 2.0 / rendered.duration_secs();
        assert!(
            (measured - played).abs() < 6.0,
            "expected about {played} Hz, measured {measured} Hz",
        );
    }

    #[test]
    fn a_silent_drip_band_renders_nothing() {
        // audio.md §5.2: the far band emits no tone update.
        let rendered = render(SoundEffect::DungeonWallDrip { band: 3 });
        assert!(rendered.samples.is_empty());
        assert!(rendered.is_silent());
    }

    #[test]
    fn every_live_effect_renders_audible_samples() {
        let effects = [
            SoundEffect::BlockedStep,
            SoundEffect::ActionSnap,
            SoundEffect::CastFailure,
            SoundEffect::DungeonWallDrip { band: 0 },
            SoundEffect::DungeonWallDrip { band: 1 },
            SoundEffect::DungeonWallDrip { band: 2 },
            SoundEffect::TrapRumble,
            SoundEffect::DamageRumble,
            SoundEffect::SharedVariant { variant: 0 },
            SoundEffect::SharedVariant { variant: 4 },
            SoundEffect::SharedVariant { variant: 8 },
            SoundEffect::Possession,
            SoundEffect::MonsterSummon,
            SoundEffect::PlayerSummon,
            SoundEffect::MoongateTransit,
            SoundEffect::StonegateDescent,
            SoundEffect::StonegateMemberDeath,
            SoundEffect::ReturnToViewStrip2,
            SoundEffect::ReturnToViewStrip3 { phase: 0 },
            SoundEffect::HarpsichordNote { digit: 6 },
            SoundEffect::EndgameRestoration,
            SoundEffect::EndgameTableau,
        ];
        for effect in effects {
            let rendered = render(effect.clone());
            assert!(!rendered.is_silent(), "{effect:?} rendered silence",);
            assert!(
                rendered.peak() <= PEAK_AMPLITUDE * 2.0,
                "{effect:?} exceeded twice the one-bit swing",
            );
        }
    }

    #[test]
    fn the_shared_variants_fall_in_pitch_and_grow_in_length() {
        // audio.md §6: the phase period falls and the iteration count rises
        // monotonically across the nine variants, so higher variants must
        // render longer and lower.
        let mut previous_duration = 0.0;
        for variant in 0..audio::SHARED_VARIANT_COUNT as u8 {
            let rendered = render(SoundEffect::SharedVariant { variant });
            assert!(
                rendered.duration_secs() > previous_duration,
                "variant {variant} must outlast its predecessor",
            );
            previous_duration = rendered.duration_secs();
        }
    }

    #[test]
    fn the_envelope_closed_form_matches_the_published_recurrence() {
        // The renderer evaluates the §5.4 recurrence in closed form; it must
        // agree with the iterative definition in `audio`.
        for segment in [
            audio::POSSESSION_ENVELOPE,
            audio::MONSTER_SUMMON_ENVELOPE,
            audio::PLAYER_SUMMON_ENVELOPE,
            audio::MOONGATE_TRANSIT_ENVELOPE,
            audio::ENDGAME_RESTORATION_ENVELOPE,
            audio::ENDGAME_TABLEAU_ENVELOPE,
            EnvelopeSegment::new(3, 2700, 512, 1, 8810),
            EnvelopeSegment::new(-1, 42_000, 512, 1, 4480),
        ] {
            let mut probe = segment;
            probe.iterations = segment.iterations.min(4096);
            for (index, expected) in probe.pin_states().enumerate() {
                assert_eq!(
                    envelope_pin_at(&probe, index as u64),
                    expected,
                    "{probe:?} iteration {index}",
                );
            }
        }
    }

    #[test]
    fn major_flash_renders_its_published_band_sweep() {
        // audio.md §8.4: 1,856 bands in 19..150 Hz.
        let mut prng = crate::prng::U5Prng::new(0x2468);
        let bands = audio::draw_major_flash_bands(&mut prng);
        let rendered = render(SoundEffect::MajorFlash { bands });
        assert!(!rendered.is_silent());
        let expected = f64::from(audio::FLASH_BAND_COUNT) * audio::FLASH_BAND_NANOS as f64 / 1.0e9;
        assert!((rendered.duration_secs() - expected).abs() < 5e-2);
    }

    #[test]
    fn wav_encoding_has_a_well_formed_header() {
        let rendered = render(SoundEffect::ActionSnap);
        let wav = wav_bytes(&rendered);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + rendered.samples.len() * 2);
        let declared = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
        assert_eq!(declared, rendered.samples.len() * 2);
    }

    #[test]
    fn renders_are_reproducible_from_the_fixed_jitter_seed() {
        let first = render(SoundEffect::TrapRumble);
        let second = render(SoundEffect::TrapRumble);
        assert_eq!(first, second);
    }
}
