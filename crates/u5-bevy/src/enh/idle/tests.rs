//! The shell half of the idle predicate and the run condition (design
//! 1.3.7).

use std::path::PathBuf;
use std::time::Duration;

use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::z_stats::{YesNoPromptKind, YesNoPromptSession};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, TileGraphicsDepth, parse_ch_font};

use super::*;
use crate::{HeldDirectionRepeat, TypedKey};

const EXPLORATION: &[IdlePromptKind] = &[IdlePromptKind::Exploration];

/// A synthetic town at its idle command prompt.
fn idle_visual_state() -> VisualState {
    let font = parse_ch_font(&vec![0x00; CH_FONT_LEN], IBM_CH_FILE).unwrap();
    VisualState {
        game_dir: PathBuf::from("."),
        state: test_state(open_grid(), 5, 5),
        atlas: synthetic_tile_atlas(TileGraphicsDepth::Ega16),
        image_handle: Handle::default(),
        text_font: font.clone(),
        rune_font: font,
        sound_effect_cursor: 0,
        prompt_cursor_frame: 0,
        input_line: String::new(),
        prompt_cursor_visible: false,
        sweep_idle_redraw_pending: false,
        held_direction_repeat: HeldDirectionRepeat::default(),
        typed_keys: Default::default(),
        flush_typeahead_when_speaker_idle: false,
        diagnostic_cursor: 0,
    }
}

fn view(visual: &VisualState) -> IdleView<'_> {
    IdleView {
        visual: Some(visual),
        speaker_playing: false,
        load_holding: false,
    }
}

#[test]
fn an_idle_town_is_idle_in_both_halves() {
    let visual = idle_visual_state();
    assert!(shell_quiescent(&visual, false, false));
    assert_eq!(view(&visual).kind(), Some(IdlePromptKind::Exploration));
    assert!(view(&visual).at(EXPLORATION));
    assert!(!view(&visual).at(&[IdlePromptKind::Combat]));
    assert!(view(&visual).still_at(EXPLORATION));
}

#[test]
fn nothing_is_idle_outside_gameplay() {
    let outside = IdleView {
        visual: None,
        speaker_playing: false,
        load_holding: false,
    };
    assert_eq!(outside.kind(), None);
    assert!(!outside.still_at(EXPLORATION));
}

/// Each shell condition refuses on its own, on a state the runtime calls
/// idle.
#[test]
fn every_shell_condition_refuses() {
    let base = idle_visual_state();
    assert_eq!(base.state.idle_blocker(), None);

    let mut queued = idle_visual_state();
    queued.typed_keys.push_back(TypedKey {
        key: KeyCode::KeyA,
        shift: false,
        control: false,
    });
    let mut redraw = idle_visual_state();
    redraw.sweep_idle_redraw_pending = true;
    let mut owed_effect = idle_visual_state();
    owed_effect.state.sound_effect_serial += 1;
    assert_ne!(
        owed_effect.sound_effect_cursor,
        owed_effect.state.sound_effect_serial
    );
    for (label, visual) in [
        ("key queued", &queued),
        ("sweep redraw", &redraw),
        ("effect not yet sent", &owed_effect),
    ] {
        assert_eq!(view(visual).kind(), None, "{label}");
    }
    let playing = IdleView {
        speaker_playing: true,
        ..view(&base)
    };
    assert_eq!(playing.kind(), None, "speaker");
    let holding = IdleView {
        load_holding: true,
        ..view(&base)
    };
    assert_eq!(holding.kind(), None, "load hold");
    assert!(!holding.still_at(EXPLORATION), "load hold");

    // The shell's own input block: a sleeping party.
    let mut asleep = idle_visual_state();
    for member in &mut asleep.state.party {
        member.status = b'S';
    }
    assert!(!shell_quiescent(&asleep, false, false));
    assert_eq!(view(&asleep).kind(), None);
}

/// Staying open needs the game at the prompt; the speaker and the queue do
/// not count against it.
#[test]
fn staying_open_is_the_weaker_test() {
    let base = idle_visual_state();
    let playing = IdleView {
        speaker_playing: true,
        ..view(&base)
    };
    assert!(playing.still_at(EXPLORATION));
    let mut prompt = idle_visual_state();
    prompt.state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    assert!(!view(&prompt).still_at(EXPLORATION));
    let mut redraw = idle_visual_state();
    redraw.sweep_idle_redraw_pending = true;
    assert!(!view(&redraw).still_at(EXPLORATION));
}

#[derive(Resource, Default)]
struct Runs(u32);

fn count(mut runs: ResMut<Runs>) {
    runs.0 += 1;
}

/// The run condition reads the real resources: the voice entity, the
/// queue's owner and the load hold.
#[test]
fn the_run_condition_reads_the_shells_resources() {
    let mut app = App::new();
    app.init_resource::<Runs>()
        .add_systems(Update, count.run_if(at_idle_prompt(EXPLORATION)));
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 0, "no gameplay: not idle");

    app.insert_resource(idle_visual_state());
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 1);

    let voice = app.world_mut().spawn(SpeakerVoice).id();
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 1, "a voice sounds");
    app.world_mut().despawn(voice);

    app.insert_resource(SpeakerQueue::default());
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 2, "an empty queue");

    app.insert_resource(EmulatorLoadLatency::new(Duration::from_millis(280)));
    app.update();
    assert_eq!(app.world().resource::<Runs>().0, 2, "the load hold");
}
