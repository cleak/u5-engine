//! Headless PC-speaker verification suite.
//!
//! Renders every published effect of `systems/audio.md` to a WAV file and
//! writes a sanitized manifest beside them. This is the audio counterpart of
//! `--save-frame-suite`: it makes the speaker contract checkable without a
//! desktop, an audio device, or the graphical shell, and the manifest is a
//! stable artefact a review can diff.
//!
//! The manifest is derived entirely from the engine's own synthesis. It
//! contains no asset bytes and no copyrighted content.

use std::fs;
use std::io;
use std::path::Path;

use u5_runtime::audio::{self, RumbleJitter, SoundEffect, SpeakerProgram};
use u5_runtime::audio_render::{RenderedSpeaker, SAMPLE_RATE, render_program, wav_bytes};
use u5_runtime::prng::U5Prng;

/// One row of the suite.
pub struct AudioSuiteCase {
    pub name: String,
    pub clause: &'static str,
    pub effect: SoundEffect,
    /// A pre-composed program, for effects that are only meaningful as a run
    /// of several boundaries rather than as one `SoundEffect`.
    pub program: Option<SpeakerProgram>,
}

impl AudioSuiteCase {
    fn lower(&self, jitter: &mut RumbleJitter) -> SpeakerProgram {
        self.program
            .clone()
            .unwrap_or_else(|| self.effect.program(jitter))
    }
}

/// Every published effect family, including the far dungeon drip band that has
/// no live caller, so the manifest covers the whole contract rather than only
/// the parts that fire.
pub fn audio_suite_cases() -> Vec<AudioSuiteCase> {
    let mut cases = vec![
        AudioSuiteCase {
            name: "blocked-step".into(),
            clause: "audio.md 7.4 165 Hz for 200 calibrated units",
            program: None,
            effect: SoundEffect::BlockedStep,
        },
        AudioSuiteCase {
            name: "action-snap".into(),
            clause: "audio.md 5.2 40 updates 1200 rising toward 2000 Hz",
            program: None,
            effect: SoundEffect::ActionSnap,
        },
        AudioSuiteCase {
            name: "cast-failure".into(),
            clause: "audio.md 5.2 50 updates 800 rising toward 2000 Hz",
            program: None,
            effect: SoundEffect::CastFailure,
        },
        AudioSuiteCase {
            name: "trap-rumble".into(),
            clause: "audio.md 5.3 75 updates 100..500 Hz",
            program: None,
            effect: SoundEffect::TrapRumble,
        },
        AudioSuiteCase {
            name: "damage-rumble".into(),
            clause: "audio.md 5.3 160 updates 100..2000 Hz",
            program: None,
            effect: SoundEffect::DamageRumble,
        },
        AudioSuiteCase {
            name: "possession".into(),
            clause: "audio.md 8.3 envelope (2, 1000, 30000, 1, 3100)",
            program: None,
            effect: SoundEffect::Possession,
        },
        AudioSuiteCase {
            name: "controlled-party-faint".into(),
            clause: "combat.md controlled-party faint possession-class envelope",
            program: None,
            effect: SoundEffect::ControlledPartyFaint,
        },
        AudioSuiteCase {
            name: "corpse-plague-rumble".into(),
            clause: "commands.md moldy-corpse plague trap-class rumble",
            program: None,
            effect: SoundEffect::CorpsePlagueRumble,
        },
        AudioSuiteCase {
            name: "sceptre-reclaimed".into(),
            clause: "audio.md 8.4.1 envelope (1, 1, 65000, 1, 4050), ~2.8 s fade at constant pitch",
            program: None,
            effect: SoundEffect::SceptreReclaimed,
        },
        AudioSuiteCase {
            name: "monster-summon".into(),
            clause: "audio.md 8.3 envelope (15, 1000, 5000, 1, 2760)",
            program: None,
            effect: SoundEffect::MonsterSummon,
        },
        AudioSuiteCase {
            name: "player-summon".into(),
            clause: "audio.md 8.3 envelope (5, 500, 12000, 1, 2760)",
            program: None,
            effect: SoundEffect::PlayerSummon,
        },
        AudioSuiteCase {
            name: "moongate-transit".into(),
            clause: "audio.md 8.3 envelope (2, 2000, 30000, 1, 5900)",
            program: None,
            effect: SoundEffect::MoongateTransit,
        },
        AudioSuiteCase {
            name: "stonegate-descent".into(),
            clause: "audio.md 8.2 750 tones 1000 down through 251 Hz",
            program: None,
            effect: SoundEffect::StonegateDescent,
        },
        AudioSuiteCase {
            name: "stonegate-member-death".into(),
            clause: "audio.md 8.2 one trap-class rumble per killed member",
            program: None,
            effect: SoundEffect::StonegateMemberDeath,
        },
        AudioSuiteCase {
            name: "return-to-view-strip2".into(),
            clause: "audio.md 8.6 rumble (20, 60, 10000), three pitches",
            program: None,
            effect: SoundEffect::ReturnToViewStrip2,
        },
        AudioSuiteCase {
            name: "return-to-view-strip3-phase0".into(),
            clause: "audio.md 8.6 3000 Hz for 3 calibrated units",
            program: None,
            effect: SoundEffect::ReturnToViewStrip3 { phase: 0 },
        },
        AudioSuiteCase {
            name: "return-to-view-strip3-phase4".into(),
            clause: "audio.md 8.6 2000 Hz for 3 calibrated units",
            program: None,
            effect: SoundEffect::ReturnToViewStrip3 { phase: 4 },
        },
        AudioSuiteCase {
            name: "blackthorn-movement-stinger".into(),
            clause: "audio.md 8.6 live 25-step rise plus 50-step fall",
            program: None,
            effect: SoundEffect::BlackthornMovementStinger,
        },
        AudioSuiteCase {
            name: "blackthorn-rescue-envelopes".into(),
            clause: "audio.md 8.6.2 six independent envelope/stop pairs",
            program: None,
            effect: SoundEffect::BlackthornRescueEnvelopes,
        },
        AudioSuiteCase {
            name: "endgame-restoration".into(),
            clause: "audio.md 8.7 envelope (1, 5000, 40000, 1, 8800)",
            program: None,
            effect: SoundEffect::EndgameRestoration,
        },
        AudioSuiteCase {
            name: "endgame-tableau".into(),
            clause: "audio.md 8.7 envelope (1, 10000, 50000, 1, 5200)",
            program: None,
            effect: SoundEffect::EndgameTableau,
        },
        AudioSuiteCase {
            name: "combat-command-refused".into(),
            clause: "audio.md 8.8 220 Hz then 150 Hz, 150 calibrated units each",
            program: None,
            effect: SoundEffect::CombatCommandRefused,
        },
        AudioSuiteCase {
            name: "long-descent".into(),
            clause: "audio.md 8.9 195 updates 660 down to a realised 272 Hz",
            program: None,
            effect: SoundEffect::LongDescent,
        },
    ];

    for band in 0..4u8 {
        cases.push(AudioSuiteCase {
            name: format!("dungeon-wall-drip-band-{band}"),
            clause: "audio.md 5.2 dungeon wall drip, 20/12/4/no updates",
            program: None,
            effect: SoundEffect::DungeonWallDrip { band },
        });
    }
    for variant in 0..audio::SHARED_VARIANT_COUNT as u8 {
        cases.push(AudioSuiteCase {
            name: format!("shared-variant-{variant}"),
            clause: "audio.md 6 rumble lead plus two opposed envelopes",
            program: None,
            effect: SoundEffect::SharedVariant { variant },
        });
    }
    // town-mode.md 13.1: the ten keys, in playing order. The scale ascends.
    for digit in [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 0] {
        cases.push(AudioSuiteCase {
            name: format!("harpsichord-key-{digit}"),
            clause: "town-mode.md 13.1 ascending scale, phase periods",
            program: None,
            effect: SoundEffect::HarpsichordNote { digit },
        });
    }

    // audio.md 8.6.1: a single dissolve click is not a meaningful artefact.
    // Its hold is about 60 microseconds, so at the recurrence's typical pitches
    // the carrier never completes a half-cycle — 8.6.1's own timbre paragraph
    // says the programmed half-period is much longer than the retune interval.
    // Rendering one click produces a sample or two of DC. What a player hears
    // is the *run*: one continuously enabled speaker retuned thousands of times.
    //
    // The gated rectangle is 320 by 101, which 8.6.1 puts at exactly 16,160
    // clicks and about one second of sound. That is rendered whole, from the
    // shipped driver state, so the WAV is the effect rather than a fragment.
    let mut dissolve_tone = audio::DissolveToneState::on_driver_load();
    let dissolve_pitches = dissolve_tone.run_for_pixels(320 * 101);
    cases.push(AudioSuiteCase {
        name: "dissolve-click-run".into(),
        clause: "audio.md 8.6.1 whole gated reveal, 16160 retunes, one stop",
        effect: SoundEffect::DissolveClick {
            frequency_hz: dissolve_pitches[0],
        },
        program: Some(audio::dissolve_click_run(&dissolve_pitches)),
    });

    // audio.md 8.4: the bands are gameplay-PRNG draws, so the manifest pins a
    // fixed seed rather than a live game state.
    let mut prng = U5Prng::new(0x1234);
    cases.push(AudioSuiteCase {
        name: "major-flash".into(),
        clause: "audio.md 8.4 1856 bands, 19..150 Hz, gameplay PRNG",
        program: None,
        effect: SoundEffect::MajorFlash {
            bands: audio::draw_major_flash_bands(&mut prng),
        },
    });
    // audio.md 7.1: one admitted burst, using the intro.md 5 pitch recurrence
    // from the published driver seed.
    let mut pitch_state = u5_runtime::subtitle_ignition::SUBTITLE_IGNITION_DRIVER_STATE_SEED;
    let pitches: Vec<u16> = (0..audio::IGNITION_BURST_PITCHES)
        .map(|_| {
            pitch_state =
                u5_runtime::subtitle_ignition::advance_subtitle_ignition_driver_state(pitch_state);
            u5_runtime::subtitle_ignition::subtitle_ignition_burst_pitch(pitch_state)
        })
        .collect();
    cases.push(AudioSuiteCase {
        name: "subtitle-ignition-burst".into(),
        clause: "audio.md 7.1 / intro.md 5, 25 pitches in 100..1500 Hz",
        program: None,
        effect: SoundEffect::SubtitleIgnitionBurst {
            pitches: pitches.into(),
        },
    });
    cases
}

/// The strongest frequency present, by discrete Fourier transform over a
/// windowed segment from the middle of the render.
///
/// This replaced a zero-crossing count, and the reason is worth recording,
/// because the old metric was worse than useless: it was **systematically
/// blind to the one defect it existed to catch**.
///
/// `audio.md §5.4`'s software envelope gates a carrier whose duty cycle sweeps
/// from about 70% to about 94% connected. As the duty approaches its extremes
/// the box-filtered waveform stops crossing zero at all, so a crossing count
/// undercounts badly and smoothly — it reported 2008 Hz for `shared-variant-0`
/// against a true spectral peak of 3125 Hz, 36% low, and was wrong in the same
/// direction for every envelope in the suite. An engine that synthesised the
/// envelope family four octaves out — the specific error `cleak/u5-spec#146`
/// warns about — would have produced a plausible-looking column here.
///
/// The square-wave families were never misread, which is exactly what made the
/// old metric convincing.
fn dominant_frequency_hz(render: &RenderedSpeaker) -> f64 {
    const WINDOW: usize = 8192;
    if render.samples.len() < 64 {
        return 0.0;
    }
    // A segment from the middle: the edge ramps at both ends are amplitude
    // envelopes, not signal, and the envelope families sweep their duty across
    // the whole render so the centre is the representative part.
    let size = WINDOW
        .min(render.samples.len().next_power_of_two() / 2)
        .max(64);
    let start = (render.samples.len() - size) / 2;
    let segment = &render.samples[start..start + size];

    // Hann window, to keep a non-integer number of periods from smearing the
    // peak across the whole spectrum.
    let mut re: Vec<f64> = segment
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (size - 1) as f64).cos();
            f64::from(*sample) * w
        })
        .collect();
    let mut im = vec![0.0_f64; size];

    // Iterative radix-2 Cooley-Tukey, so the suite carries no FFT dependency.
    let mut j = 0usize;
    for i in 1..size {
        let mut bit = size >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= size {
        let angle = -std::f64::consts::TAU / len as f64;
        let (wr, wi) = (angle.cos(), angle.sin());
        for block in (0..size).step_by(len) {
            let (mut cr, mut ci) = (1.0_f64, 0.0_f64);
            for k in 0..len / 2 {
                let (ur, ui) = (re[block + k], im[block + k]);
                let (vr0, vi0) = (re[block + k + len / 2], im[block + k + len / 2]);
                let (vr, vi) = (vr0 * cr - vi0 * ci, vr0 * ci + vi0 * cr);
                re[block + k] = ur + vr;
                im[block + k] = ui + vi;
                re[block + k + len / 2] = ur - vr;
                im[block + k + len / 2] = ui - vi;
                let next = (cr * wr - ci * wi, cr * wi + ci * wr);
                cr = next.0;
                ci = next.1;
            }
        }
        len <<= 1;
    }

    // Skip the DC bin and its immediate neighbour: the DC blocker leaves a
    // residual on effects shorter than its time constant, and that residual is
    // not the effect's pitch.
    let mut best = (0usize, 0.0_f64);
    for bin in 2..size / 2 {
        let magnitude = re[bin] * re[bin] + im[bin] * im[bin];
        if magnitude > best.1 {
            best = (bin, magnitude);
        }
    }
    best.0 as f64 * f64::from(render.sample_rate) / size as f64
}

fn digest(samples: &[f32]) -> u64 {
    // FNV-1a over the quantized sample stream, so the manifest is stable
    // across platforms with the same arithmetic.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for sample in samples {
        let quantized = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        for byte in quantized.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// Render every case into `out_dir` and write `manifest.txt`.
pub fn run_audio_suite(out_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;
    // Clear stale renders before writing. The published effect inventory has
    // changed repeatedly during this work — effects have been added, renamed
    // and withdrawn — and a WAV left behind by an earlier run is
    // indistinguishable from a current one when someone opens the directory to
    // listen. Four superseded single-click renders survived this way and were
    // only caught by an external waveform check.
    for entry in fs::read_dir(out_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "wav") {
            fs::remove_file(path)?;
        }
    }
    let mut jitter = RumbleJitter::new();
    let mut manifest = String::new();
    manifest.push_str("# PC-speaker suite, rendered from systems/audio.md\n");
    manifest.push_str(&format!("# sample rate: {SAMPLE_RATE} Hz, mono, 16-bit\n"));
    manifest.push_str(&format!(
        "# calibration anchor (cleak/u5-spec#146): one outer calibrated unit = {:.3} ms,\n         # one inner unit = {:.1} us, nominal boot calibration = {}\n",
        audio::OUTER_CALIBRATED_UNIT_NANOS as f64 / 1.0e6,
        audio::INNER_UNIT_NANOS as f64 / 1.0e3,
        audio::NOMINAL_BOOT_CALIBRATION,
    ));
    manifest.push_str(
        "# muted_seconds differs from seconds only for the software envelope, which\n         # cleak/u5-spec#146 confirms is NOT cost-matched when silent.\n",
    );
    manifest
        .push_str("name\tops\ttones\tseconds\tmuted_seconds\tpeak\tdominant_hz\tdigest\tclause\n");

    let cases = audio_suite_cases();
    let mut silent = Vec::new();
    for case in &cases {
        let program = case.lower(&mut jitter);
        let render = render_program(&program, true);
        let path = out_dir.join(format!("{}.wav", case.name));
        fs::write(&path, wav_bytes(&render))?;
        manifest.push_str(&format!(
            "{}\t{}\t{}\t{:.4}\t{:.4}\t{:.3}\t{:.1}\t{:016x}\t{}\n",
            case.name,
            program.ops.len(),
            program.tone_count(),
            render.duration_secs(),
            program.total_nanos(false) as f64 / 1.0e9,
            render.peak(),
            dominant_frequency_hz(&render),
            digest(&render.samples),
            case.clause,
        ));
        if render.is_silent() {
            silent.push(case.name.clone());
        }
    }

    fs::write(out_dir.join("manifest.txt"), &manifest)?;
    println!(
        "Wrote {} PC-speaker WAVs and a manifest to {}",
        cases.len(),
        out_dir.display()
    );
    // audio.md 5.2: exactly one case is silent by contract, the far dungeon
    // drip band, whose negative span emits no tone update and only stops.
    if silent != ["dungeon-wall-drip-band-3"] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected only the far dungeon drip band to render silence, got {silent:?}"),
        ));
    }
    println!("Silent by contract: dungeon-wall-drip-band-3 (audio.md 5.2 negative span)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The suite must cover every published effect family.
    ///
    /// Asserted by *family* rather than by total count. A bare
    /// `cases.len() == N` breaks every time the specification publishes another
    /// effect — which it has done repeatedly during this work — and the fix is
    /// always to bump N, which is a change that cannot fail for the right
    /// reason. Naming the families instead means a newly-published effect only
    /// breaks this test if it belongs to a family and was forgotten.
    #[test]
    fn the_suite_covers_every_published_effect_family() {
        let cases = audio_suite_cases();
        let names: Vec<&str> = cases.iter().map(|case| case.name.as_str()).collect();

        let present = |needle: &str| names.iter().any(|name| name.contains(needle));
        for family in [
            "blocked-step",
            "action-snap",
            "cast-failure",
            "trap-rumble",
            "damage-rumble",
            "possession",
            "controlled-party-faint",
            "corpse-plague-rumble",
            "monster-summon",
            "player-summon",
            "moongate-transit",
            "stonegate-descent",
            "stonegate-member-death",
            "return-to-view-strip2",
            "return-to-view-strip3",
            "blackthorn-movement-stinger",
            "blackthorn-rescue-envelopes",
            "endgame-restoration",
            "endgame-tableau",
            "dissolve-click",
            "major-flash",
            "subtitle-ignition-burst",
        ] {
            assert!(present(family), "the suite is missing the {family} family");
        }

        // audio.md 5.2: all four dungeon drip depth bands, including the far
        // band whose negative span emits no tone.
        for band in 0..4u8 {
            assert!(present(&format!("dungeon-wall-drip-band-{band}")));
        }
        // audio.md 6: all nine shared potion/wind/spell variants.
        for variant in 0..audio::SHARED_VARIANT_COUNT {
            assert!(present(&format!("shared-variant-{variant}")));
        }
        // town-mode.md 13.1: all ten harpsichord keys.
        for digit in 0..10u8 {
            assert!(present(&format!("harpsichord-key-{digit}")));
        }

        let mut sorted = names.clone();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "case names must be unique");
    }

    /// `audio.md §2`: an implementation "must stop the speaker at every
    /// specified effect end". The visual shell honours that by giving its one
    /// voice exactly `SpeakerProgram::duration` and then retiring it, so every
    /// render has to land on its own program's duration to within one sample.
    /// A longer render is a tail the speaker can never reach; a shorter one is
    /// a voice holding an exhausted sink.
    ///
    /// This is the whole published inventory, in both sound states. It is the
    /// assertion that would have caught the per-operation rounding that made
    /// `§8.6.1`'s gated reveal render 12.44% long and `§7.1`'s ignition burst
    /// 7.25% long.
    #[test]
    fn every_case_renders_to_its_own_program_duration() {
        let sample_secs = 1.0 / f64::from(SAMPLE_RATE);
        for audible in [true, false] {
            let mut jitter = RumbleJitter::new();
            for case in audio_suite_cases() {
                let program = case.lower(&mut jitter);
                let rendered = render_program(&program, audible);
                let program_secs = program.total_nanos(audible) as f64 / 1.0e9;
                let error = rendered.duration_secs() - program_secs;
                assert!(
                    error.abs() <= sample_secs,
                    "{} audible={audible}: rendered {:.6} s against a {:.6} s program ({:+.3}%)",
                    case.name,
                    rendered.duration_secs(),
                    program_secs,
                    error / program_secs * 100.0,
                );
            }
        }
    }

    #[test]
    fn only_the_far_drip_band_renders_silence() {
        let mut jitter = RumbleJitter::new();
        for case in audio_suite_cases() {
            let render = render_program(&case.lower(&mut jitter), true);
            let expected_silent = case.name == "dungeon-wall-drip-band-3";
            assert_eq!(
                render.is_silent(),
                expected_silent,
                "{} silence mismatch",
                case.name,
            );
        }
    }

    /// `town-mode.md §13.1`: the scale **ascends** — digit `1` is the lowest
    /// note and the instrument spans a major tenth in natural left-to-right
    /// order. The previously published descending reading is withdrawn, and
    /// this suite asserted it until the correction landed.
    /// The envelope families must render at their published pitches.
    ///
    /// This is the assertion the suite was missing. `cleak/u5-spec#146` Q4
    /// warns that a frontend synthesising the envelope as a pin toggle at the
    /// loop rate lands "about four octaves wrong", and until now nothing here
    /// could have caught that: the manifest's pitch column was a zero-crossing
    /// count, which reads ~36% low on a duty-swept waveform and would have
    /// shown a plausible number for a badly wrong render.
    #[test]
    fn the_envelope_families_render_at_their_published_pitches() {
        let mut jitter = RumbleJitter::new();
        let measure = |name: &str, jitter: &mut RumbleJitter| {
            let case = audio_suite_cases()
                .into_iter()
                .find(|case| case.name == name)
                .unwrap_or_else(|| panic!("suite has no case named {name}"));
            dominant_frequency_hz(&render_program(&case.lower(jitter), true))
        };

        // audio.md 8.3 and 8.7 name these outright.
        for (name, published) in [
            ("possession", 1101.0),
            ("moongate-transit", 2096.0),
            ("endgame-restoration", 3126.0),
            ("endgame-tableau", 1847.0),
        ] {
            let hz = measure(name, &mut jitter);
            let error = (hz - published).abs() / published;
            assert!(
                error < 0.03,
                "{name} measured {hz} Hz against a published {published} Hz",
            );
        }

        // audio.md 6: the nine shared variants form a clean descending octave,
        // about 3130 Hz down to about 1592 Hz.
        let top = measure("shared-variant-0", &mut jitter);
        let bottom = measure("shared-variant-8", &mut jitter);
        assert!(
            (2800.0..=3350.0).contains(&top),
            "variant 0 measured {top} Hz against a published 2.8..3.35 kHz",
        );
        assert!(
            (1450.0..=1750.0).contains(&bottom),
            "variant 8 measured {bottom} Hz against a published ~1592 Hz",
        );
        let ratio = top / bottom;
        assert!(
            (1.90..2.05).contains(&ratio),
            "the family must span one octave, measured {ratio}",
        );
    }

    #[test]
    fn the_harpsichord_keys_render_in_ascending_pitch_order() {
        let mut jitter = RumbleJitter::new();
        let mut previous = 0.0_f64;
        for digit in [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 0] {
            let render = render_program(
                &SoundEffect::HarpsichordNote { digit }.program(&mut jitter),
                true,
            );
            let measured = dominant_frequency_hz(&render);
            assert!(
                measured > previous,
                "key {digit} measured {measured} Hz, which is not above {previous} Hz",
            );
            previous = measured;
        }
    }
}
