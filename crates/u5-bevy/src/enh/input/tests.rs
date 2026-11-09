//! The allocation proof (design 1.3.6) and the router's behaviour.

use std::path::PathBuf;

use bevy::input::keyboard::{Key, NativeKey};
use u5_runtime::enhancements::{
    FeatureToggles, LockReason, LockState, ResolvedEnhancements, TOGGLES,
};
use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
use u5_runtime::{CH_FONT_LEN, IBM_CH_FILE, TileGraphicsDepth, parse_ch_font};

use super::*;
use crate::enh::{EnhancementsPlugin, VisualLaunchConfig, add_enhancements};
use crate::{AnimationPump, HeldDirectionRepeat, SpeakerEffect, TypedKey, drive_visual};

const ROUTER: &str = "enh_input_router";

/// The ten `[keys]` defaults, as the resolver hands them over.
fn default_keys() -> KeyBindings {
    ResolvedEnhancements::classic().keys
}

/// Every `[keys]` default bound, as if every feature were on.
fn all_defaults_bound() -> KeyMap {
    let (map, notes) = KeyMap::build(default_keys().iter(), |_| true);
    assert!(notes.is_empty(), "{notes:?}");
    map
}

fn chord(text: &str) -> KeyChord {
    KeyChord::parse(text).unwrap_or_else(|| panic!("{text} parses"))
}

/// Both mapping functions give `None` for `chord`, with its modifiers.
/// They take no Alt, so this is also the check with Alt ignored.
fn game_ignores(chord: KeyChord) -> bool {
    key_code_to_input_byte(chord.key, chord.shift, chord.ctrl).is_none()
        && key_code_to_line_input_byte(chord.key, chord.shift, chord.ctrl).is_none()
}

/// Every modifier spelling a chord can carry, with its flags.
const MODIFIERS: [(&str, bool, bool, bool); 8] = [
    ("", false, false, false),
    ("Ctrl+", true, false, false),
    ("Shift+", false, true, false),
    ("Alt+", false, false, true),
    ("Ctrl+Shift+", true, true, false),
    ("Ctrl+Alt+", true, false, true),
    ("Shift+Alt+", false, true, true),
    ("Ctrl+Shift+Alt+", true, true, true),
];

// ---------------------------------------------------------------------------
// The allocation proof.

/// Proof item 1, defaults: no default chord maps to a byte in either
/// function, with its modifiers and with Alt added.
#[test]
fn no_default_chord_is_a_key_the_game_reads() {
    let keys = default_keys();
    assert_eq!(keys.iter().count(), 10);
    for (name, text) in keys.iter() {
        let text = text.unwrap_or_else(|| panic!("{name} has a default"));
        let parsed = chord(text);
        assert!(game_ignores(parsed), "{name} = {text}");
        assert!(
            game_ignores(KeyChord {
                alt: true,
                ..parsed
            }),
            "{name} = Alt+{text}"
        );
        assert!(!parsed.reserved(), "{name} = {text}");
        assert!(EnhAction::from_config_key(name).is_some(), "{name}");
    }
}

/// Proof item 1, every chord a player could write: the map binds a chord
/// only when neither function maps it, and refuses exactly the ones they
/// map.
#[test]
fn every_bindable_chord_is_one_the_game_ignores() {
    let mut bound = 0;
    for (name, key) in KEY_NAMES {
        for (prefix, ctrl, shift, alt) in MODIFIERS {
            let text = format!("{prefix}{name}");
            let expected = KeyChord {
                key: *key,
                ctrl,
                shift,
                alt,
            };
            assert_eq!(KeyChord::parse(&text), Some(expected), "{text}");
            let (map, notes) = KeyMap::build([("keys.menu", Some(text.as_str()))], |_| true);
            if game_ignores(expected) {
                assert_eq!(map.action(expected), Some(EnhAction::OpenMenu), "{text}");
                assert!(notes.is_empty(), "{text}: {notes:?}");
                bound += 1;
            } else {
                assert!(map.is_empty(), "{text} is a key the game reads");
                assert_eq!(
                    notes,
                    [KeyMapNote::UsedByGame {
                        key: "keys.menu",
                        chord: text.clone(),
                    }]
                );
            }
        }
    }
    assert!(bound > 0);
}

/// The free and reserved sets of 1.3.6, spelled out.
#[test]
fn the_free_keys_are_the_ones_the_design_lists() {
    for free in [
        "F11",
        "F12",
        "Tab",
        "Insert",
        "Delete",
        "Pause",
        "ScrollLock",
        "Ctrl+J",
        "Ctrl+P",
        "Ctrl+M",
        "Ctrl+A",
        "Ctrl+Shift+A",
        "Ctrl+F",
        "Ctrl+F12",
        "Ctrl+Shift+J",
    ] {
        assert!(!chord(free).reserved(), "{free} is free");
    }
    let mut reserved = vec![
        "A",
        "Z",
        "5",
        "Shift+1",
        "Space",
        "Enter",
        "NumpadEnter",
        "Backspace",
        "Escape",
        "Up",
        "Down",
        "Left",
        "Right",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "Numpad8",
        "NumpadAdd",
        "Minus",
        "Slash",
        "Alt+Enter",
        "Alt+A",
    ];
    let function_keys: Vec<String> = (1..=10).map(|n| format!("F{n}")).collect();
    reserved.extend(function_keys.iter().map(String::as_str));
    // The five control chords the 1.0.1 mapping delivers; Shift does not
    // change a Ctrl chord's result.
    for letter in ["E", "S", "K", "V", "B"] {
        assert!(chord(&format!("Ctrl+{letter}")).reserved(), "Ctrl+{letter}");
        assert!(
            chord(&format!("Ctrl+Shift+{letter}")).reserved(),
            "Ctrl+Shift+{letter}"
        );
    }
    for text in reserved {
        assert!(chord(text).reserved(), "{text} is the game's");
    }
}

/// Proof item 2: a rebinding onto a key the game reads is refused with a
/// note, and so is a chord that names no key.
#[test]
fn a_rebinding_onto_a_game_key_is_refused_with_a_note() {
    for text in ["Ctrl+E", "F1", "Alt+Enter", "Escape", "A", "PageUp"] {
        let (map, notes) = KeyMap::build([("keys.journal", Some(text))], |_| true);
        assert!(map.is_empty(), "{text}");
        let note = KeyMapNote::UsedByGame {
            key: "keys.journal",
            chord: text.to_string(),
        };
        assert_eq!(notes, std::slice::from_ref(&note));
        assert!(note.to_string().starts_with(KEY_USED_BY_GAME), "{note}");
        assert!(
            note.to_string()
                .ends_with(&format!("keys.journal = {text}"))
        );
    }
    for text in ["", "Hyper+Q", "Ctrl+Ctrl+J", "Ctrl+", "ShiftLeft", "F25"] {
        let (map, notes) = KeyMap::build([("keys.journal", Some(text))], |_| true);
        assert!(map.is_empty(), "{text:?}");
        assert_eq!(
            notes,
            [KeyMapNote::NotUnderstood {
                key: "keys.journal",
                chord: text.to_string(),
            }]
        );
    }
}

/// Proof item 3: no two enabled bindings share a chord. The first action
/// in catalogue order keeps it; an action that is off takes no chord.
#[test]
fn no_two_enabled_bindings_share_a_chord() {
    let map = all_defaults_bound();
    assert_eq!(map.iter().count(), 10, "the ten defaults are distinct");

    let entries = [("keys.menu", Some("F12")), ("keys.map", Some("f12"))];
    let (map, notes) = KeyMap::build(entries, |_| true);
    assert_eq!(map.action(chord("F12")), Some(EnhAction::OpenMenu));
    assert_eq!(map.iter().count(), 1);
    let note = KeyMapNote::Taken {
        key: "keys.map",
        chord: "f12".to_string(),
        by: "keys.menu",
    };
    assert_eq!(notes, std::slice::from_ref(&note));
    assert!(note.to_string().starts_with(KEY_TAKEN));

    let (map, notes) = KeyMap::build(entries, |action| action != EnhAction::OpenMenu);
    assert_eq!(map.action(chord("F12")), Some(EnhAction::OpenMap));
    assert!(notes.is_empty(), "a binding that is off holds nothing");
}

/// Proof item 5: the contextual gem pair is the only binding on reserved
/// keys, held only while the gem map asks for it, and kept over a rebuild.
#[test]
fn only_the_contextual_gem_pair_sits_on_reserved_keys() {
    let mut map = all_defaults_bound();
    assert!(map.iter().all(|(chord, _)| !chord.reserved()));
    assert_eq!(EnhAction::GemPageUp.config_key(), None);
    assert_eq!(EnhAction::GemPageDown.config_key(), None);

    map.set_gem_paging(true);
    let reserved: Vec<EnhAction> = map
        .iter()
        .filter(|(chord, _)| chord.reserved())
        .map(|(_, action)| action)
        .collect();
    assert_eq!(reserved.len(), 2);
    assert_eq!(map.action(chord("PageUp")), Some(EnhAction::GemPageUp));
    assert_eq!(map.action(chord("PageDown")), Some(EnhAction::GemPageDown));
    assert_eq!(map.action(chord("Shift+PageUp")), None);

    let keys = default_keys();
    map.rebuild(&keys, FeatureToggles::CLASSIC);
    assert_eq!(map.iter().count(), 2, "only the contextual pair is left");
    map.set_gem_paging(false);
    assert!(map.is_empty());
}

#[test]
fn chords_parse_without_regard_to_case_or_modifier_order() {
    let expected = KeyChord {
        key: KeyCode::KeyA,
        ctrl: true,
        shift: true,
        alt: false,
    };
    for text in ["Ctrl+Shift+A", "shift+ctrl+a", "Control + Shift + A"] {
        assert_eq!(KeyChord::parse(text), Some(expected), "{text}");
    }
    assert_eq!(KeyChord::parse("tab"), Some(KeyChord::new(KeyCode::Tab)));
    assert_eq!(KeyChord::parse("Esc"), Some(KeyChord::new(KeyCode::Escape)));
}

/// Bindings follow their feature. No feature row has landed in the
/// catalogue yet, so nothing binds in this build; the parked minimap and
/// the contextual pair never bind through `[keys]`.
#[test]
fn bindings_follow_their_features() {
    let mut every = FeatureToggles::CLASSIC;
    for spec in TOGGLES {
        every.set(spec.toggle, true);
    }
    for action in EnhAction::ALL {
        assert!(!action.enabled(every), "{action:?}");
    }
    let (map, notes) = KeyMap::for_toggles(&default_keys(), every);
    assert!(map.is_empty());
    assert!(notes.is_empty());
    assert!(!menu_overlay_on(every));

    let modal: Vec<EnhAction> = EnhAction::ALL
        .into_iter()
        .filter(|action| action.is_modal())
        .collect();
    assert_eq!(
        modal,
        [
            EnhAction::OpenMenu,
            EnhAction::OpenMap,
            EnhAction::OpenJournal,
            EnhAction::OpenCheats,
            EnhAction::GemPageUp,
            EnhAction::GemPageDown,
        ]
    );
}

// ---------------------------------------------------------------------------
// Zero footprint (proof item 4).

fn has_system(
    app: &mut App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
    name: &str,
) -> bool {
    app.world_mut()
        .resource_mut::<bevy::ecs::schedule::Schedules>()
        .get(schedule)
        .is_some_and(|schedule| {
            schedule
                .graph()
                .systems()
                .any(|(_, system, _)| system.name().contains(name))
        })
}

#[test]
fn no_router_while_nothing_is_bound() {
    // Classic and the lock: no plugin at all.
    for launch in [
        VisualLaunchConfig::classic(),
        VisualLaunchConfig {
            enhancements: ResolvedEnhancements::locked(LockReason::CliFlag),
            ..VisualLaunchConfig::classic()
        },
    ] {
        let mut app = App::new();
        add_enhancements(&mut app, &launch);
        assert!(!app.world().contains_resource::<KeyMap>());
        assert!(!app.world().contains_resource::<KeyLatch>());
        assert!(!has_system(&mut app, PreUpdate, ROUTER));
    }

    // A toggle on whose feature binds no key: the plugin, but no router.
    let mut launch = VisualLaunchConfig::classic();
    launch
        .enhancements
        .toggles
        .set(Toggle::EnhancementsFreezeOverlays, true);
    assert_eq!(launch.enhancements.lock, LockState::Off);
    let mut app = App::new();
    add_enhancements(&mut app, &launch);
    assert!(app.is_plugin_added::<EnhancementsPlugin>());
    assert!(!app.world().contains_resource::<KeyMap>());
    assert!(!app.world().contains_resource::<KeyLatch>());
    assert!(!app.world().contains_resource::<Events<EnhActionEvent>>());
    assert!(!has_system(&mut app, PreUpdate, ROUTER));
    app.update();
}

#[test]
fn a_bound_key_installs_the_router_in_its_slot() {
    let mut app = App::new();
    install_input_router(&mut app, all_defaults_bound());
    assert!(has_system(&mut app, PreUpdate, ROUTER));
    assert!(!has_system(&mut app, Update, ROUTER));
    assert!(app.world().contains_resource::<KeyLatch>());
}

// ---------------------------------------------------------------------------
// The router in a headless app.

/// A headless app with the router: the keyboard resource, its events and
/// the bindings, pressed the way the shell's own harness does.
struct Keys {
    app: App,
}

impl Keys {
    fn new(map: KeyMap) -> Self {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default())
            .add_event::<KeyboardInput>()
            .add_event::<KeyboardFocusLost>();
        install_input_router(&mut app, map);
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

    /// Runs one frame and returns what the game would see as just pressed.
    fn frame(&mut self) -> Vec<KeyCode> {
        self.app.update();
        let world = self.app.world_mut();
        let mut keyboard = world.resource_mut::<ButtonInput<KeyCode>>();
        let mut seen: Vec<KeyCode> = keyboard.get_just_pressed().copied().collect();
        seen.sort_by_key(|key| format!("{key:?}"));
        keyboard.clear();
        seen
    }

    fn pressed(&self, key: KeyCode) -> bool {
        self.app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(key)
    }

    fn events<E: Event + Copy>(&self) -> Vec<E> {
        self.app
            .world()
            .resource::<Events<E>>()
            .iter_current_update_events()
            .copied()
            .collect()
    }

    fn actions(&self) -> Vec<EnhAction> {
        self.events::<EnhActionEvent>()
            .into_iter()
            .map(|event| event.0)
            .collect()
    }

    fn set_modal(&mut self, open: bool) {
        self.app
            .world_mut()
            .resource_mut::<KeyLatch>()
            .set_modal(open);
    }
}

#[test]
fn a_bound_chord_is_consumed_and_an_unbound_key_left_alone() {
    let mut keys = Keys::new(all_defaults_bound());
    keys.press(KeyCode::F12);
    assert_eq!(keys.frame(), [], "F12 never reaches the game");
    assert_eq!(keys.actions(), [EnhAction::OpenMenu]);
    assert!(!keys.pressed(KeyCode::F12));
    keys.release(KeyCode::F12);
    keys.frame();

    for unbound in [KeyCode::Insert, KeyCode::KeyA, KeyCode::F5] {
        keys.press(unbound);
        assert_eq!(keys.frame(), [unbound], "{unbound:?} reaches the game");
        assert_eq!(keys.actions(), []);
        assert!(keys.pressed(unbound));
        keys.release(unbound);
        keys.frame();
    }

    // Ctrl chords: Ctrl itself is left alone, the letter is consumed.
    keys.press(KeyCode::ControlLeft);
    assert_eq!(keys.frame(), [KeyCode::ControlLeft]);
    keys.press(KeyCode::KeyJ);
    assert_eq!(keys.frame(), []);
    assert_eq!(keys.actions(), [EnhAction::OpenJournal]);
    keys.press(KeyCode::ShiftRight);
    keys.press(KeyCode::KeyA);
    assert_eq!(keys.frame(), [KeyCode::ShiftRight]);
    assert_eq!(keys.actions(), [EnhAction::AutoParty]);
    // Ctrl+E is the game's.
    keys.press(KeyCode::KeyE);
    assert_eq!(keys.frame(), [KeyCode::KeyE]);
    assert_eq!(keys.actions(), []);
}

#[test]
fn a_modal_hotkey_must_be_the_first_key_of_the_frame() {
    let mut keys = Keys::new(all_defaults_bound());
    keys.press(KeyCode::KeyA);
    keys.press(KeyCode::F12);
    assert_eq!(keys.frame(), [KeyCode::F12, KeyCode::KeyA], "both go on");
    assert_eq!(keys.actions(), []);
    assert_eq!(
        keys.events::<HotkeyFellThrough>(),
        [HotkeyFellThrough(EnhAction::OpenMenu)]
    );
    for key in [KeyCode::KeyA, KeyCode::F12] {
        keys.release(key);
    }
    keys.frame();

    // A plain toggle is not modal and is taken wherever it falls.
    keys.press(KeyCode::KeyA);
    keys.press(KeyCode::F11);
    assert_eq!(keys.frame(), [KeyCode::KeyA]);
    assert_eq!(keys.actions(), [EnhAction::ToggleFullscreen]);
    for key in [KeyCode::KeyA, KeyCode::F11] {
        keys.release(key);
    }
    keys.frame();

    // A modifier typed in the same frame is not a key ahead of it.
    keys.press(KeyCode::ControlLeft);
    keys.press(KeyCode::KeyJ);
    assert_eq!(keys.frame(), [KeyCode::ControlLeft]);
    assert_eq!(keys.actions(), [EnhAction::OpenJournal]);
}

#[test]
fn os_repeat_of_a_consumed_hotkey_asks_for_nothing_more() {
    let mut keys = Keys::new(all_defaults_bound());
    keys.press(KeyCode::F11);
    keys.frame();
    assert_eq!(keys.actions(), [EnhAction::ToggleFullscreen]);
    for _ in 0..3 {
        keys.repeat(KeyCode::F11);
        assert_eq!(keys.frame(), [], "the repeat is consumed too");
        assert_eq!(keys.actions(), [], "and toggles nothing");
    }
    keys.release(KeyCode::F11);
    keys.frame();
    keys.press(KeyCode::F11);
    keys.frame();
    assert_eq!(keys.actions(), [EnhAction::ToggleFullscreen]);
}

#[test]
fn while_modal_every_key_belongs_to_the_overlay() {
    let mut keys = Keys::new(all_defaults_bound());
    keys.press(KeyCode::F12);
    keys.frame();
    assert_eq!(keys.actions(), [EnhAction::OpenMenu]);
    // The overlay opens; F12 is still held.
    keys.set_modal(true);
    keys.repeat(KeyCode::F12);
    assert_eq!(keys.frame(), []);
    assert_eq!(
        keys.events::<OverlayKey>(),
        [],
        "the held opener is latched"
    );
    keys.release(KeyCode::F12);
    keys.press(KeyCode::ShiftLeft);
    keys.press(KeyCode::KeyA);
    keys.press(KeyCode::ArrowUp);
    assert_eq!(
        keys.frame(),
        [KeyCode::ShiftLeft],
        "only the modifier stays"
    );
    assert_eq!(
        keys.events::<OverlayKey>(),
        [OverlayKey(KeyCode::KeyA), OverlayKey(KeyCode::ArrowUp)]
    );
    assert_eq!(keys.actions(), []);
    assert!(keys.pressed(KeyCode::ShiftLeft));
    assert!(!keys.pressed(KeyCode::ArrowUp), "a held arrow never walks");
    // OS repeat of the held arrow: forwarded, still never pressed.
    keys.repeat(KeyCode::ArrowUp);
    assert_eq!(keys.frame(), []);
    assert_eq!(keys.events::<OverlayKey>(), [OverlayKey(KeyCode::ArrowUp)]);
    assert!(!keys.pressed(KeyCode::ArrowUp));
    // A bound key goes to the overlay too.
    keys.press(KeyCode::Tab);
    keys.frame();
    assert_eq!(keys.events::<OverlayKey>(), [OverlayKey(KeyCode::Tab)]);
    assert_eq!(keys.actions(), []);
    keys.release(KeyCode::Tab);
    keys.release(KeyCode::KeyA);
    keys.frame();

    // The overlay closes with the arrow still held: ignored until released.
    keys.set_modal(false);
    keys.repeat(KeyCode::ArrowUp);
    assert_eq!(keys.frame(), []);
    assert!(
        keys.app
            .world()
            .resource::<KeyLatch>()
            .is_latched(KeyCode::ArrowUp)
    );
    keys.repeat(KeyCode::ArrowUp);
    assert_eq!(keys.frame(), []);
    keys.release(KeyCode::ArrowUp);
    keys.frame();
    keys.press(KeyCode::ArrowUp);
    assert_eq!(
        keys.frame(),
        [KeyCode::ArrowUp],
        "a fresh press is the game's"
    );
}

#[test]
fn losing_focus_lets_go_of_the_latch() {
    let mut keys = Keys::new(all_defaults_bound());
    keys.press(KeyCode::KeyQ);
    keys.frame();
    keys.set_modal(true);
    keys.frame();
    keys.set_modal(false);
    keys.frame();
    assert!(
        keys.app
            .world()
            .resource::<KeyLatch>()
            .is_latched(KeyCode::KeyQ)
    );
    keys.app.world_mut().send_event(KeyboardFocusLost);
    keys.frame();
    assert!(
        !keys
            .app
            .world()
            .resource::<KeyLatch>()
            .is_latched(KeyCode::KeyQ)
    );
}

// ---------------------------------------------------------------------------
// Equivalence with Classic through the real input driver.

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

/// The `--visual` input driver alone on [`idle_visual_state`], with or
/// without the router.
fn driver_app(map: Option<KeyMap>) -> Keys {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .insert_resource(ButtonInput::<KeyCode>::default())
        .insert_resource(Assets::<Image>::default())
        .insert_resource(AnimationPump::default())
        .insert_resource(idle_visual_state())
        .add_event::<KeyboardInput>()
        .add_event::<SpeakerEffect>()
        .add_event::<AppExit>()
        .add_systems(Update, drive_visual);
    if let Some(map) = map {
        install_input_router(&mut app, map);
    }
    Keys { app }
}

impl Keys {
    /// What the game holds: its queue, its line and its state.
    fn game_view(&self) -> String {
        let visual = self.app.world().resource::<VisualState>();
        format!(
            "{:?} {:?} {:?} {:?}",
            visual.typed_keys, visual.input_line, visual.held_direction_repeat, visual.state
        )
    }

    /// Taps `key` (with Ctrl held around it) over two frames and returns
    /// the game's view after each frame, and the actions sent.
    fn tap(&mut self, key: KeyCode, ctrl: bool) -> (Vec<String>, Vec<EnhAction>) {
        let mut views = Vec::new();
        if ctrl {
            self.press(KeyCode::ControlLeft);
        }
        self.press(key);
        self.frame();
        views.push(self.game_view());
        let actions = if self.app.world().contains_resource::<KeyMap>() {
            self.actions()
        } else {
            Vec::new()
        };
        self.release(key);
        if ctrl {
            self.release(KeyCode::ControlLeft);
        }
        self.frame();
        views.push(self.game_view());
        (views, actions)
    }
}

/// F11, F12, Tab, Ctrl+J and Ctrl+P leave the game's queue and state as
/// Classic leaves them: Classic queues the free key and discards it at the
/// read, the router takes it before the driver.
#[test]
fn hotkeys_leave_the_game_as_classic_does() {
    let mut classic = driver_app(None);
    let mut routed = driver_app(Some(all_defaults_bound()));
    classic.frame();
    routed.frame();
    assert_eq!(classic.game_view(), routed.game_view());
    for (key, ctrl, action) in [
        (KeyCode::F11, false, EnhAction::ToggleFullscreen),
        (KeyCode::F12, false, EnhAction::OpenMenu),
        (KeyCode::Tab, false, EnhAction::OpenMap),
        (KeyCode::KeyJ, true, EnhAction::OpenJournal),
        (KeyCode::KeyP, true, EnhAction::ToggleColumn),
    ] {
        let (classic_views, _) = classic.tap(key, ctrl);
        let (routed_views, actions) = routed.tap(key, ctrl);
        assert_eq!(classic_views, routed_views, "{key:?}");
        assert_eq!(actions, [action], "{key:?}");
    }
}

/// A key still queued from an earlier frame is a key ahead of the hotkey:
/// the menu key falls through to the game, which drops it.
#[test]
fn a_queued_key_sends_a_modal_hotkey_to_the_game() {
    let mut visual = idle_visual_state();
    visual.typed_keys.push_back(TypedKey {
        key: KeyCode::KeyX,
        shift: false,
        control: false,
    });
    let mut keys = Keys::new(all_defaults_bound());
    keys.app.insert_resource(visual);
    keys.press(KeyCode::F12);
    assert_eq!(keys.frame(), [KeyCode::F12]);
    assert_eq!(keys.actions(), []);
    assert_eq!(
        keys.events::<HotkeyFellThrough>(),
        [HotkeyFellThrough(EnhAction::OpenMenu)]
    );
    // Only modifiers queued: nothing is ahead of it.
    keys.release(KeyCode::F12);
    keys.frame();
    keys.app
        .world_mut()
        .resource_mut::<VisualState>()
        .typed_keys = [TypedKey {
        key: KeyCode::ShiftLeft,
        shift: true,
        control: false,
    }]
    .into();
    keys.press(KeyCode::F12);
    assert_eq!(keys.frame(), []);
    assert_eq!(keys.actions(), [EnhAction::OpenMenu]);
}
