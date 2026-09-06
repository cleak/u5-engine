//! Print the sound effects a scripted turn emits, in order.
//!
//! A capture answers "what came out of the speaker"; this answers "what
//! did the engine ask for", which is the other half of any audio
//! divergence. `PlayState` keeps a bounded history of emitted effects
//! with a serial number, so replaying a `--play-script` and dumping it
//! shows the order the shell will play them in.

use std::path::Path;
use u5_runtime::*;
use u5_tui::{replay_play_script_commands, split_play_script};

/// The variant name alone, without its payload.
fn effect_name(effect: &SoundEffect) -> String {
    let debug = format!("{effect:?}");
    match debug.split_once([' ', '{']) {
        Some((name, _)) => name.trim().to_string(),
        None => debug,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: sound_history_probe <PROFILE_DIR> [SCRIPT]");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let mut state = PlayState::load_scene(dir, options).expect("scene must load");
    if let Some(script) = args.next() {
        let commands = split_play_script(&script);
        replay_play_script_commands(&mut state, dir, &commands, |_, _, _| Ok(())).expect("replay");
    }
    let effects = state.sound_effects_after(0);
    println!("{} effect(s):", effects.len());
    let mut jitter = audio::RumbleJitter::new();
    let mut total = 0.0;
    for (index, effect) in effects.iter().enumerate() {
        let seconds = effect.program(&mut jitter).duration(true).as_secs_f64();
        total += seconds;
        // `MajorFlash` carries its 1,856 drawn bands; printing them buries
        // the report, so effects are named rather than debugged.
        println!("  {index:>3} {:<28} {seconds:.3}s", effect_name(effect));
    }
    println!("total {total:.3}s if played serially");
}
