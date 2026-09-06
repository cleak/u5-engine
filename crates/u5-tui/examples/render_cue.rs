//! Render one published sound cue to a WAV, for comparison against a capture.
//!
//! Some cues are only reachable through a stochastic trigger - the long
//! descent of `audio.md §8.9` needs a frigate sunk by a wandering
//! encounter - so waiting for both sides to hit one inside the same
//! paired run is impractical. The two implementations can still be
//! compared: record the stock game's cue whenever it happens, render
//! ours here, and put the two waveforms side by side.
//!
//! The rendering is the same `render_program` the Bevy shell plays
//! through, so this is our speaker output, not an idealisation of it.

use std::io::Write;
use u5_runtime::audio::{RumbleJitter, SoundEffect};
use u5_runtime::audio_render::{render_program, wav_bytes};

fn main() {
    let mut args = std::env::args().skip(1);
    let name = args.next().expect("usage: render_cue <CUE> <OUT.wav>");
    let out = args.next().expect("usage: render_cue <CUE> <OUT.wav>");
    let effect = match name.as_str() {
        "long-descent" => SoundEffect::LongDescent,
        "blocked-step" => SoundEffect::BlockedStep,
        "stonegate-descent" => SoundEffect::StonegateDescent,
        "combat-refused" => SoundEffect::CombatCommandRefused,
        other => panic!("unknown cue {other}"),
    };
    let program = effect.program(&mut RumbleJitter::new());
    let rendered = render_program(&program, true);
    println!(
        "{name}: {:.3}s, {} samples at {} Hz",
        rendered.duration_secs(),
        rendered.samples.len(),
        rendered.sample_rate
    );
    std::fs::File::create(&out)
        .and_then(|mut file| file.write_all(&wav_bytes(&rendered)))
        .expect("write the wav");
    println!("wrote {out}");
}
