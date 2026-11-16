//! Phase 4d, fast-forward (design 6.3.4, 6.4.3; plan M7-07 commit 1): in
//! "until done" with `fast_forward` on, the blocking cues of an auto key's
//! span play a fixed short envelope in their own order; anything else, and
//! the speaker with no shell at all, is the 1.0.1 playback.

use u5_runtime::audio::{InterruptibleSound, RumbleJitter, SoundEffect};
use u5_runtime::audio_render::render_program;

use super::engaged_tests::{fast, game_in, weak_arena};
use super::shell_tests::Chord;
use super::*;
use crate::enh::equivalence::Scene;
use crate::{
    SpeakerEffect, SpeakerJitter, SpeakerVoiceLifetime, VisualSoundWave, play_speaker_effects,
};

/// Long enough to be cut, and of three families: two tones, a rumble and
/// a blocked-step click.
fn cues() -> Vec<SoundEffect> {
    vec![
        SoundEffect::CombatCommandRefused,
        SoundEffect::DamageRumble,
        SoundEffect::BlockedStep,
        SoundEffect::CombatCommandRefused,
    ]
}

/// The speaker alone, as the window has it, with `shell` as the resource
/// (or none).
fn speaker_app(shell: Option<AutoCombatShell>) -> App {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .init_resource::<Assets<VisualSoundWave>>()
        .init_resource::<SpeakerJitter>()
        .init_resource::<SpeakerQueue>()
        .add_event::<SpeakerEffect>()
        .add_systems(Update, play_speaker_effects);
    if let Some(shell) = shell {
        app.insert_resource(shell);
    }
    app
}

/// One voice as the speaker started it: its lifetime and its samples.
#[derive(Debug, PartialEq)]
struct Voice {
    lifetime: Duration,
    samples: Vec<f32>,
}

/// Sends every event in one frame, then lets each voice end at once, and
/// returns the voices in the order they sounded.
fn play(shell: Option<AutoCombatShell>, events: Vec<SpeakerEffect>) -> Vec<Voice> {
    let mut app = speaker_app(shell);
    let count = events.len();
    for event in events {
        app.world_mut().send_event(event);
    }
    let mut voices = Vec::new();
    for _ in 0..count {
        app.update();
        let mut query = app.world_mut().query::<(
            Entity,
            &SpeakerVoiceLifetime,
            Option<&AudioPlayer<VisualSoundWave>>,
        )>();
        let found: Vec<(Entity, Duration, Option<Handle<VisualSoundWave>>)> = query
            .iter(app.world())
            .map(|(entity, lifetime, player)| {
                (
                    entity,
                    lifetime.0.duration(),
                    player.map(|player| player.0.clone()),
                )
            })
            .collect();
        assert_eq!(found.len(), 1, "one voice at a time");
        let (entity, lifetime, handle) = found.into_iter().next().unwrap();
        let samples = handle.map_or_else(Vec::new, |handle| {
            app.world()
                .resource::<Assets<VisualSoundWave>>()
                .get(&handle)
                .unwrap()
                .samples
                .to_vec()
        });
        voices.push(Voice { lifetime, samples });
        app.world_mut().despawn(entity);
    }
    voices
}

fn effects(cues: &[SoundEffect], interruptible: Option<InterruptibleSound>) -> Vec<SpeakerEffect> {
    cues.iter()
        .map(|effect| SpeakerEffect {
            interruptible,
            ..SpeakerEffect::new(effect.clone(), true)
        })
        .collect()
}

/// A shell in "until done" with `fast_forward` as given and an auto key's
/// span open.
fn running(fast_forward: bool) -> AutoCombatShell {
    let mut shell = AutoCombatShell::new(CombatSettings {
        auto: true,
        fast_forward,
        ..CombatSettings::default()
    });
    shell.engaged = AutoEngagement {
        members: [true; 6],
        until_done: true,
    };
    shell.open_cue_span(0);
    shell
}

#[test]
fn fast_forward_cuts_each_cue_to_the_short_envelope_in_the_same_order() {
    let normal = play(None, effects(&cues(), None));
    let fast = play(Some(running(true)), effects(&cues(), None));
    // The programs as the speaker builds them, against one jitter stream.
    let mut jitter = RumbleJitter::default();
    let nanos = FAST_FORWARD_CUE.as_nanos() as u64;
    assert_eq!(fast.len(), normal.len());
    for (index, effect) in cues().iter().enumerate() {
        let program = effect.program(&mut jitter);
        assert_eq!(normal[index].lifetime, program.duration(true), "{effect:?}");
        let short = program.truncated_at(nanos, true);
        assert_eq!(fast[index].lifetime, short.duration(true), "{effect:?}");
        assert!(fast[index].lifetime <= FAST_FORWARD_CUE, "{effect:?}");
        assert!(fast[index].lifetime < normal[index].lifetime, "{effect:?}");
        // The same cue, in the same place: only its length changed.
        assert_eq!(
            fast[index].samples,
            render_program(&short, true).samples,
            "{effect:?}"
        );
        let head = fast[index].samples.len().min(normal[index].samples.len()) / 2;
        assert_eq!(
            fast[index].samples[..head],
            normal[index].samples[..head],
            "{effect:?}"
        );
    }
}

#[test]
fn outside_fast_forward_every_cue_plays_as_in_1_0_1() {
    let normal = play(None, effects(&cues(), None));
    let off = running(false);
    let mut party = running(true);
    party.engaged.until_done = false;
    let mut quiet = running(true);
    quiet.close_cue_span(3);
    let mut idle = running(true);
    idle.cue_span = None;
    for (name, shell) in [
        ("fast_forward off", off),
        ("party, not until done", party),
        ("span closed", quiet),
        ("no key fed", idle),
    ] {
        assert!(!shell.fast_forwards_cues(), "{name}");
        assert_eq!(play(Some(shell), effects(&cues(), None)), normal, "{name}");
    }
    // An interruptible program is not a blocking cue: its own cancel ends it.
    let tagged = effects(&cues(), Some(InterruptibleSound::GatedDissolve));
    assert_eq!(
        play(Some(running(true)), tagged.clone()),
        play(None, tagged)
    );
}

#[test]
fn each_auto_key_records_its_serial_span_and_fast_forward_shortens_the_fight() {
    let mut game = game_in(weak_arena(), &Scene::synthetic(), |settings| {
        fast(settings);
        settings.fast_forward = true;
    });
    game.frames(2);
    let before = game.shell.visual().state.sound_effect_serial;
    game.engage(Chord::UntilDone);
    let span = game.auto().unwrap().cue_span.expect("the fed key's span");
    assert_eq!(span.after, before);
    let (mut shortened, mut spans) = (std::collections::HashSet::new(), Vec::<CueSpan>::new());
    for _ in 0..2000 {
        game.frames(1);
        let auto = game.auto().unwrap();
        if let Some(span) = auto.cue_span {
            match spans.last_mut() {
                Some(last) if last.after == span.after => *last = span,
                _ => spans.push(span),
            }
        }
        // A voice that sounds while the span is open was started in it.
        let fast = auto.fast_forwards_cues();
        let world = game.shell.app.world_mut();
        let mut voices = world.query::<(Entity, &SpeakerVoiceLifetime)>();
        for (entity, lifetime) in voices.iter(world) {
            if fast {
                assert!(lifetime.0.duration() <= FAST_FORWARD_CUE);
                shortened.insert(entity);
            }
        }
        let auto = game.auto().unwrap();
        if !auto.engaged.any() && auto.handing_back.is_none() {
            break;
        }
    }
    assert_eq!(game.toast(), Some(AUTO_OFF_VICTORY));
    assert!(!shortened.is_empty(), "the fight's cues were cut");
    // One span per fed key, each closed where the shell was quiet before
    // the next key (in the next key's own frame when nothing was left to
    // wait for, so this loop saw it open).
    assert!(spans.len() >= 3, "{spans:?}");
    for pair in spans.windows(2) {
        let through = pair[0].through.unwrap_or(pair[1].after);
        assert!(
            pair[0].after <= through && through <= pair[1].after,
            "{spans:?}"
        );
    }
}
