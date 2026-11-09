//! The overlay model through the real input driver and idle pump, with a
//! test-only overlay (design 1.3.7, plan M0-12).

use std::path::PathBuf;
use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardFocusLost, KeyboardInput, NativeKey};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use u5_runtime::enhancements::{LockReason, ResolvedEnhancements, TOGGLES, Toggle};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::z_stats::{YesNoPromptKind, YesNoPromptSession};
use u5_runtime::{
    CH_FONT_LEN, COMBAT_ACTOR_FLAG_SELECTABLE_80, CombatActorDescriptor, IBM_CH_FILE,
    TileGraphicsDepth, parse_ch_font,
};

use super::*;
use crate::enh::input::{EnhActionEvent, OverlayKey};
use crate::enh::{VisualLaunchConfig, add_enhancements, configure_enh_sets};
use crate::{
    AnimationPump, HeldDirectionRepeat, SpeakerEffect, SpeakerVoice, TypedKey,
    VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH, animate_static_tiles, drive_visual,
};

const FRAME: Duration = Duration::from_micros(16_667);

/// The test-only overlay: the map key opens the map, Escape closes what is
/// open. It asks through the model like a real feature.
fn test_overlay(
    mut actions: EventReader<EnhActionEvent>,
    mut keys: EventReader<OverlayKey>,
    state: Res<OverlayState>,
    mut requests: EventWriter<OverlayRequest>,
) {
    for action in actions.read() {
        if action.0 == EnhAction::OpenMap {
            requests.write(OverlayRequest::Open(OverlayId::Map));
        }
    }
    for key in keys.read() {
        if key.0 == KeyCode::Escape
            && let Some(id) = state.open()
        {
            requests.write(OverlayRequest::Close(id));
        }
    }
}

/// A synthetic town at its idle command prompt, drawing into a real image.
fn idle_visual_state(images: &mut Assets<Image>) -> VisualState {
    let font = parse_ch_font(&vec![0x00; CH_FONT_LEN], IBM_CH_FILE).unwrap();
    let image_handle = images.add(Image::new(
        Extent3d {
            width: VISUAL_PLAY_FRAME_WIDTH,
            height: VISUAL_PLAY_FRAME_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (VISUAL_PLAY_FRAME_WIDTH * VISUAL_PLAY_FRAME_HEIGHT * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    VisualState {
        game_dir: PathBuf::from("."),
        state: test_state(open_grid(), 5, 5),
        atlas: synthetic_tile_atlas(TileGraphicsDepth::Ega16),
        image_handle,
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

/// The gameplay input driver and idle pump on [`idle_visual_state`], under
/// manual time; with `overlay`, the router with every default bound, the
/// overlay model and the test overlay.
struct Game {
    app: App,
}

impl Game {
    fn new(overlay: bool) -> Self {
        let mut images = Assets::<Image>::default();
        let visual = idle_visual_state(&mut images);
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(Time::<bevy::prelude::Real>::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(images)
            .insert_resource(AnimationPump::default())
            .insert_resource(visual)
            .add_event::<KeyboardInput>()
            .add_event::<KeyboardFocusLost>()
            .add_event::<SpeakerEffect>()
            .add_event::<AppExit>()
            .add_systems(Update, (drive_visual, animate_static_tiles).chain());
        if overlay {
            configure_enh_sets(&mut app);
            let (keymap, notes) =
                KeyMap::build(ResolvedEnhancements::classic().keys.iter(), |_| true);
            assert!(notes.is_empty());
            install_input_router(&mut app, keymap);
            install_overlay_model(&mut app);
            app.add_systems(
                Update,
                test_overlay
                    .in_set(EnhSet::Apply)
                    .before(apply_overlay_requests),
            );
        }
        Self { app }
    }

    fn event(&mut self, key_code: KeyCode, state: ButtonState, repeat: bool) {
        let world = self.app.world_mut();
        let mut keyboard = world.resource_mut::<ButtonInput<KeyCode>>();
        match state {
            ButtonState::Pressed => keyboard.press(key_code),
            ButtonState::Released => keyboard.release(key_code),
        }
        world.send_event(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat,
            window: Entity::PLACEHOLDER,
        });
    }

    fn press(&mut self, key: KeyCode) {
        self.event(key, ButtonState::Pressed, false);
    }

    fn repeat(&mut self, key: KeyCode) {
        self.event(key, ButtonState::Pressed, true);
    }

    fn release(&mut self, key: KeyCode) {
        self.event(key, ButtonState::Released, false);
    }

    /// One frame of manual time, as the shell's harness steps it.
    fn frame(&mut self) {
        let world = self.app.world_mut();
        world.resource_mut::<Time>().advance_by(FRAME);
        world
            .resource_mut::<Time<bevy::prelude::Real>>()
            .update_with_duration(FRAME);
        self.app.update();
        self.app
            .world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }

    fn visual(&self) -> &VisualState {
        self.app.world().resource::<VisualState>()
    }

    fn visual_mut(&mut self) -> Mut<'_, VisualState> {
        self.app.world_mut().resource_mut::<VisualState>()
    }

    /// The game's state, its queue and the frame it shows.
    fn game_view(&self) -> String {
        let visual = self.visual();
        let image = self
            .app
            .world()
            .resource::<Assets<Image>>()
            .get(&visual.image_handle)
            .and_then(|image| image.data.clone())
            .unwrap_or_default();
        format!(
            "{:?} {:?} {} {} {:?} {:?}",
            visual.state,
            visual.typed_keys,
            visual.prompt_cursor_frame,
            visual.prompt_cursor_visible,
            visual.held_direction_repeat,
            image
        )
    }

    /// The Avatar's cell, the turn and the message: what a step changes.
    fn where_the_avatar_is(&self) -> (usize, usize, u64, String) {
        let state = &self.visual().state;
        (
            state.player.x,
            state.player.y,
            state.turn,
            state.message.clone(),
        )
    }

    fn open(&self) -> Option<OverlayId> {
        self.app.world().resource::<OverlayState>().open()
    }

    fn events<E: Event + Copy>(&self) -> Vec<E> {
        self.app
            .world()
            .resource::<Events<E>>()
            .iter_current_update_events()
            .copied()
            .collect()
    }

    fn request(&mut self, request: OverlayRequest) {
        self.app.world_mut().send_event(request);
    }

    /// Taps the map key: the test overlay asks for the map.
    fn tap_map_key(&mut self) {
        self.press(KeyCode::Tab);
        self.frame();
        self.release(KeyCode::Tab);
    }
}

fn has_system(app: &mut App, name: &str) -> bool {
    app.world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .iter()
        .any(|(_, schedule)| {
            schedule
                .graph()
                .systems()
                .any(|(_, system, _)| system.name().contains(name))
        })
}

// ---------------------------------------------------------------------------
// Zero footprint.

#[test]
fn no_overlay_model_while_no_overlay_feature_is_on() {
    for launch in [
        VisualLaunchConfig::classic(),
        VisualLaunchConfig {
            enhancements: ResolvedEnhancements::locked(LockReason::QaEnv),
            ..VisualLaunchConfig::classic()
        },
    ] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<OverlayState>());
    }
    // Every catalogue toggle on: none of them is an overlay feature yet.
    let mut launch = VisualLaunchConfig::classic();
    for spec in TOGGLES {
        launch.enhancements.toggles.set(spec.toggle, true);
    }
    assert!(!overlay_feature_on(launch.enhancements.toggles));
    let mut app = App::new();
    add_enhancements(&mut app, &launch);
    assert!(app.is_plugin_added::<crate::enh::EnhancementsPlugin>());
    let world = app.world();
    assert!(!world.contains_resource::<OverlayState>());
    assert!(!world.contains_resource::<Events<OverlayRequest>>());
    assert!(!world.contains_resource::<Events<OverlayChanged>>());
    assert!(!has_system(&mut app, "apply_overlay_requests"));
    assert!(!has_system(&mut app, "close_when_not_idle"));
    // An overlay feature binds its opener; the model follows it.
    assert_eq!(Toggle::from_key("automap.fullscreen_map"), None);
}

#[test]
fn the_model_brings_the_router_it_needs() {
    let mut app = App::new();
    install_overlay_model(&mut app);
    assert!(app.world().contains_resource::<KeyLatch>());
    assert!(app.world().resource::<KeyMap>().is_empty());
    assert!(has_system(&mut app, "enh_input_router"));
    assert!(has_system(&mut app, "apply_overlay_requests"));
    assert!(has_system(&mut app, "close_when_not_idle"));
    let state = app.world().resource::<OverlayState>();
    assert!(!state.is_open());
    assert!(!state.frozen(), "Live in v1 (U1)");
}

// ---------------------------------------------------------------------------
// Where overlays open.

#[test]
fn an_overlay_opens_at_the_idle_exploration_prompt() {
    let mut game = Game::new(true);
    game.frame();
    game.tap_map_key();
    assert_eq!(game.open(), Some(OverlayId::Map));
    assert_eq!(
        game.events::<OverlayChanged>(),
        [OverlayChanged::Opened(OverlayId::Map)]
    );
    assert!(game.app.world().resource::<KeyLatch>().is_modal());
    assert!(!game.app.world().resource::<OverlayState>().frozen());
}

/// Design 1.3.7: refusal in each excluded state, runtime and shell halves.
#[test]
fn an_overlay_is_refused_away_from_the_idle_prompt() {
    let refusals: [(&str, fn(&mut Game)); 5] = [
        ("yes/no prompt", |game| {
            game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
                kind: YesNoPromptKind::SaveGame,
            });
        }),
        ("key queued", |game| {
            game.visual_mut().typed_keys.push_back(TypedKey {
                key: KeyCode::KeyX,
                shift: false,
                control: false,
            });
        }),
        ("speaker voice", |game| {
            game.app.world_mut().spawn(SpeakerVoice);
        }),
        ("idle redraw owed", |game| {
            game.visual_mut().sweep_idle_redraw_pending = true;
        }),
        ("party asleep", |game| {
            for member in &mut game.visual_mut().state.party {
                member.status = b'S';
            }
        }),
    ];
    for (label, setup) in refusals {
        let mut game = Game::new(true);
        game.frame();
        setup(&mut game);
        game.request(OverlayRequest::Open(OverlayId::Map));
        game.app.update();
        assert_eq!(game.open(), None, "{label}");
        assert_eq!(
            game.events::<OverlayRefused>(),
            [OverlayRefused(OverlayId::Map)],
            "{label}"
        );
        assert!(
            !game.app.world().resource::<KeyLatch>().is_modal(),
            "{label}"
        );
    }
    for id in OverlayId::ALL {
        assert!(id.refusal().ends_with("prompt."), "{id:?}");
    }
}

#[test]
fn only_the_cheats_page_opens_at_a_combat_prompt() {
    let mut game = Game::new(true);
    {
        let mut visual = game.visual_mut();
        let state = &mut visual.state;
        state.combat_active = true;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.pending_combat_actor_slot = Some(0);
    }
    for id in [OverlayId::Menu, OverlayId::Journal, OverlayId::Map] {
        game.request(OverlayRequest::Open(id));
        game.app.update();
        assert_eq!(game.open(), None, "{id:?}");
        assert_eq!(game.events::<OverlayRefused>(), [OverlayRefused(id)]);
    }
    game.request(OverlayRequest::Open(OverlayId::Cheats));
    game.app.update();
    assert_eq!(game.open(), Some(OverlayId::Cheats));

    // The automatic walk is not a member's prompt: it closes.
    game.visual_mut().state.pending_combat_actor_slot = None;
    game.app.update();
    assert_eq!(game.open(), None);
}

#[test]
fn opening_one_overlay_closes_the_other() {
    let mut game = Game::new(true);
    game.frame();
    game.tap_map_key();
    assert_eq!(game.open(), Some(OverlayId::Map));
    game.request(OverlayRequest::Open(OverlayId::Menu));
    game.app.update();
    assert_eq!(game.open(), Some(OverlayId::Menu));
    assert_eq!(
        game.events::<OverlayChanged>(),
        [
            OverlayChanged::Closed {
                id: OverlayId::Map,
                why: CloseReason::Replaced,
            },
            OverlayChanged::Opened(OverlayId::Menu),
        ]
    );
    // Asking again for the open one changes nothing.
    game.request(OverlayRequest::Open(OverlayId::Menu));
    game.app.update();
    assert_eq!(game.events::<OverlayChanged>(), []);
    // Closing one that is not open changes nothing either.
    game.request(OverlayRequest::Close(OverlayId::Map));
    game.app.update();
    assert_eq!(game.open(), Some(OverlayId::Menu));
}

// ---------------------------------------------------------------------------
// Keys while open.

/// Keys never reach `typed_keys` while open, OS repeat and the close frame
/// included, and a held arrow never moves the Avatar.
#[test]
fn keys_never_reach_the_game_while_an_overlay_is_open() {
    let mut game = Game::new(true);
    game.frame();
    game.tap_map_key();
    assert_eq!(game.open(), Some(OverlayId::Map));
    let start = game.where_the_avatar_is();

    // A held arrow with OS repeat for well past the repeat delay.
    game.press(KeyCode::ArrowUp);
    for frame in 0..90 {
        game.frame();
        assert!(game.visual().typed_keys.is_empty(), "frame {frame}");
        assert_eq!(
            game.visual().held_direction_repeat,
            HeldDirectionRepeat::default(),
            "frame {frame}"
        );
        if frame % 3 == 0 {
            game.repeat(KeyCode::ArrowUp);
        }
    }
    for key in [KeyCode::KeyA, KeyCode::Space, KeyCode::Enter, KeyCode::F12] {
        game.press(key);
        game.frame();
        game.release(key);
        game.frame();
        assert!(game.visual().typed_keys.is_empty(), "{key:?}");
    }
    assert_eq!(game.where_the_avatar_is(), start);

    // Escape closes it, with the arrow still held: not in the close frame,
    // not on repeat after it.
    game.press(KeyCode::Escape);
    game.frame();
    assert_eq!(game.open(), None);
    assert_eq!(
        game.events::<OverlayChanged>(),
        [OverlayChanged::Closed {
            id: OverlayId::Map,
            why: CloseReason::Requested,
        }]
    );
    assert!(game.visual().typed_keys.is_empty(), "the close frame");
    for _ in 0..30 {
        game.repeat(KeyCode::ArrowUp);
        game.repeat(KeyCode::Escape);
        game.frame();
        assert!(game.visual().typed_keys.is_empty());
    }
    let latch = game.app.world().resource::<KeyLatch>();
    assert!(latch.is_latched(KeyCode::ArrowUp));
    assert!(latch.is_latched(KeyCode::Escape));
    assert_eq!(game.where_the_avatar_is(), start);

    // Released and pressed again, the arrow is the game's.
    game.release(KeyCode::ArrowUp);
    game.release(KeyCode::Escape);
    game.frame();
    game.press(KeyCode::ArrowUp);
    game.frame();
    assert_ne!(game.where_the_avatar_is(), start, "a fresh press moves");
}

/// Keys typed in the same frame behind the hotkey belong to the overlay
/// that hotkey opened.
#[test]
fn keys_behind_the_opening_hotkey_stay_away_from_the_game() {
    let mut game = Game::new(true);
    game.frame();
    let start = game.where_the_avatar_is();
    game.press(KeyCode::Tab);
    game.press(KeyCode::ArrowUp);
    game.frame();
    assert_eq!(game.open(), Some(OverlayId::Map));
    assert!(game.visual().typed_keys.is_empty());
    assert_eq!(game.where_the_avatar_is(), start);
    for _ in 0..40 {
        game.repeat(KeyCode::ArrowUp);
        game.frame();
    }
    assert_eq!(game.where_the_avatar_is(), start);
}

#[test]
fn closing_releases_the_held_direction() {
    let mut game = Game::new(true);
    game.frame();
    game.tap_map_key();
    game.visual_mut().held_direction_repeat = {
        let mut held = HeldDirectionRepeat::default();
        held.press(KeyCode::ArrowLeft);
        held
    };
    game.request(OverlayRequest::Close(OverlayId::Map));
    game.app.update();
    assert_eq!(game.open(), None);
    assert_eq!(
        game.visual().held_direction_repeat,
        HeldDirectionRepeat::default()
    );
}

// ---------------------------------------------------------------------------
// World time while open.

/// An idle tick under a Live overlay is real time: the run with the overlay
/// open, keys typed into it, equals the run with no plugin and no keys,
/// frame for frame.
#[test]
fn a_live_overlay_leaves_the_idle_ticks_as_the_off_run_has_them() {
    let mut off = Game::new(false);
    let mut on = Game::new(true);
    off.frame();
    on.frame();
    assert_eq!(off.game_view(), on.game_view());
    on.press(KeyCode::Tab);
    for frame in 0..240 {
        match frame {
            1 => on.release(KeyCode::Tab),
            10 | 50 | 120 => on.press(KeyCode::KeyN),
            11 | 51 | 121 => on.release(KeyCode::KeyN),
            60 => on.press(KeyCode::ArrowDown),
            61..=100 => on.repeat(KeyCode::ArrowDown),
            101 => on.release(KeyCode::ArrowDown),
            _ => {}
        }
        off.frame();
        on.frame();
        if frame >= 1 {
            assert_eq!(on.open(), Some(OverlayId::Map), "frame {frame}");
        }
        assert_eq!(off.game_view(), on.game_view(), "frame {frame}");
    }
    assert!(
        off.visual().prompt_cursor_frame > 0,
        "the idle pump ran in both"
    );
}

// ---------------------------------------------------------------------------
// Closing when the prompt stops being idle.

#[test]
fn the_overlay_closes_when_the_prompt_stops_being_idle() {
    let mut game = Game::new(true);
    game.frame();
    game.tap_map_key();
    assert_eq!(game.open(), Some(OverlayId::Map));
    game.visual_mut().state.active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    game.frame();
    assert_eq!(game.open(), None);
    assert_eq!(
        game.events::<OverlayChanged>(),
        [OverlayChanged::Closed {
            id: OverlayId::Map,
            why: CloseReason::NotIdle,
        }]
    );
    assert!(!game.app.world().resource::<KeyLatch>().is_modal());
}

/// An ambient effect from the idle steps refuses an opening but does not
/// close an overlay that is open.
#[test]
fn a_speaker_voice_does_not_close_an_open_overlay() {
    let mut game = Game::new(true);
    game.frame();
    game.tap_map_key();
    let voice = game.app.world_mut().spawn(SpeakerVoice).id();
    for _ in 0..10 {
        game.frame();
    }
    assert_eq!(game.open(), Some(OverlayId::Map));
    game.app.world_mut().despawn(voice);
    game.frame();
    assert_eq!(game.open(), Some(OverlayId::Map));
}
