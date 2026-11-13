//! The hotkey router and the key allocation (Phase B design 1.2.5, 1.2.6
//! and 1.3.6).
//!
//! One system, [`enh_input_router`], runs in `PreUpdate` after Bevy's input
//! system ([`EnhSet::Input`]). That is before every faithful `Update`
//! system of both app chains and the intro, so it needs no edge into
//! faithful code. It is in the app only while some binding is active (or
//! the menu overlay is on, or the gem map's level paging may bind PgUp and
//! PgDn, D-M3.6); with nothing bound there is no [`KeyMap`], no
//! [`KeyLatch`] and no router.
//!
//! **Allocation.** The shell turns keys into game bytes in
//! `key_code_to_input_byte` and, for typed lines,
//! `key_code_to_line_input_byte`. A chord either of them maps is reserved
//! and never bound ([`KeyChord::reserved`]). Every other key reaches the
//! input driver, is queued, and is discarded at the read with nothing
//! flushed and no turn spent, so taking it away before the driver leaves
//! the game's input trace unchanged. The router reads those functions and
//! `keys_pressed_in_typed_order`; it never edits them.
//!
//! **Consuming a key.** For a bound chord the router sends an
//! [`EnhActionEvent`] and resets the key on `ButtonInput`. The input driver
//! builds its typed keys from events filtered on `just_pressed`, so a reset
//! key never reaches the game's queue or the held-direction repeat.
//!
//! **The order rule (1.2.6).** A modal hotkey ([`EnhAction::is_modal`]) is
//! honoured only if it is the first key typed in the frame and nothing is
//! queued ahead of it (modifier keys, which map to no byte, do not count). Otherwise the key is left alone, the game drops it
//! exactly as 1.0.1 does, and a [`HotkeyFellThrough`] lets the toast say
//! why. Key order is never changed.
//!
//! **Modal input.** While an overlay is open ([`KeyLatch::set_modal`]) the
//! router resets every key every frame (OS key repeat presses a reset key
//! again) and forwards each press as an [`OverlayKey`]. Modifier keys are
//! left alone: they map to no byte, and the game must see them as they are
//! when the overlay closes. Keys held when an overlay opens or closes are
//! ignored until released.

use std::collections::{HashMap, HashSet};
use std::fmt;

use bevy::ecs::event::EventCursor;
use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyboardFocusLost, KeyboardInput};
use bevy::prelude::*;
use u5_runtime::enhancements::{CHEATS_SWITCH, FeatureToggles, KeyBindings, Toggle};

use super::EnhSet;
use crate::{
    VisualState, key_code_to_input_byte, key_code_to_line_input_byte, keys_pressed_in_typed_order,
};

/// Logged when a `[keys]` chord does not name a key.
const KEY_NOT_UNDERSTOOD: &str = "keys: the key name was not understood, so the action has no key";
/// Logged when a `[keys]` chord is one the game itself reads (no shadow
/// bindings in v1, U3).
const KEY_USED_BY_GAME: &str = "keys: the game uses this key itself, so the action has no key";
/// Logged when two actions name the same chord; the first one keeps it.
const KEY_TAKEN: &str = "keys: another action already has this key, so the action has no key";

/// What a hotkey asks for. Feature systems read these as
/// [`EnhActionEvent`]s and never read the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnhAction {
    OpenMenu,
    ToggleFullscreen,
    OpenMap,
    ToggleMinimap,
    OpenJournal,
    ToggleColumn,
    AutoMember,
    AutoParty,
    AutoUntilDone,
    OpenCheats,
    /// Contextual (chapter 10): bound only while a one-panel dungeon gem
    /// map is open; never a `[keys]` entry.
    GemPageUp,
    /// See [`EnhAction::GemPageUp`].
    GemPageDown,
}

impl EnhAction {
    /// Every action, the `[keys]` ones in catalogue order.
    pub const ALL: [EnhAction; 12] = [
        EnhAction::OpenMenu,
        EnhAction::ToggleFullscreen,
        EnhAction::OpenMap,
        EnhAction::ToggleMinimap,
        EnhAction::OpenJournal,
        EnhAction::ToggleColumn,
        EnhAction::AutoMember,
        EnhAction::AutoParty,
        EnhAction::AutoUntilDone,
        EnhAction::OpenCheats,
        EnhAction::GemPageUp,
        EnhAction::GemPageDown,
    ];

    /// The `[keys]` entry that binds this action. `None` for the contextual
    /// gem paging pair, which cannot be rebound.
    pub fn config_key(self) -> Option<&'static str> {
        Some(match self {
            EnhAction::OpenMenu => "keys.menu",
            EnhAction::ToggleFullscreen => "keys.fullscreen",
            EnhAction::OpenMap => "keys.map",
            EnhAction::ToggleMinimap => "keys.minimap",
            EnhAction::OpenJournal => "keys.journal",
            EnhAction::ToggleColumn => "keys.column",
            EnhAction::AutoMember => "keys.auto_member",
            EnhAction::AutoParty => "keys.auto_party",
            EnhAction::AutoUntilDone => "keys.auto_until_done",
            EnhAction::OpenCheats => "keys.cheats",
            EnhAction::GemPageUp | EnhAction::GemPageDown => return None,
        })
    }

    /// The action a `[keys]` entry binds.
    pub fn from_config_key(key: &str) -> Option<EnhAction> {
        EnhAction::ALL
            .into_iter()
            .find(|action| action.config_key() == Some(key))
    }

    /// Whether the action opens a modal overlay or pages one. These follow
    /// the order rule of 1.2.6; the others are honoured whenever pressed.
    pub fn is_modal(self) -> bool {
        matches!(
            self,
            EnhAction::OpenMenu
                | EnhAction::OpenMap
                | EnhAction::OpenJournal
                | EnhAction::OpenCheats
                | EnhAction::GemPageUp
                | EnhAction::GemPageDown
        )
    }

    /// The feature toggles whose being on binds this action (1.2.5). A
    /// feature whose rows have not landed in the catalogue yet binds
    /// nothing.
    fn feature_keys(self) -> &'static [&'static str] {
        match self {
            EnhAction::OpenMenu => &["menu.overlay", "menu.title_layer"],
            EnhAction::ToggleFullscreen => &["display.fullscreen_key"],
            EnhAction::OpenMap => &["automap.fullscreen_map"],
            EnhAction::OpenJournal => &["journal.page", "journal.clues", "journal.quest_tracker"],
            EnhAction::ToggleColumn => &["display.grow_left"],
            EnhAction::AutoMember | EnhAction::AutoParty | EnhAction::AutoUntilDone => {
                &["combat.auto"]
            }
            // The minimap is parked (U18): its chord is reserved and never
            // bound. Cheats are any effective cheat (below). The gem pair is
            // contextual.
            EnhAction::ToggleMinimap
            | EnhAction::OpenCheats
            | EnhAction::GemPageUp
            | EnhAction::GemPageDown => &[],
        }
    }

    /// Whether the features behind this action are on, so its `[keys]`
    /// chord is bound.
    pub fn enabled(self, toggles: FeatureToggles) -> bool {
        match self {
            // Any effective cheat, the read-only state watch included. The
            // resolver has already turned off every state-altering cheat
            // without the section switch.
            EnhAction::OpenCheats => toggles
                .iter_on()
                .any(|toggle| toggle.key().starts_with("cheats.") && toggle.key() != CHEATS_SWITCH),
            _ => self
                .feature_keys()
                .iter()
                .filter_map(|key| Toggle::from_key(key))
                .any(|toggle| toggles.on(toggle)),
        }
    }
}

/// A key with its modifiers. Left and right modifiers count the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub key: KeyCode,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// The key names a `[keys]` chord may use, matched without regard to case.
/// Modifier keys are not here: a chord needs a key.
const KEY_NAMES: &[(&str, KeyCode)] = &[
    ("A", KeyCode::KeyA),
    ("B", KeyCode::KeyB),
    ("C", KeyCode::KeyC),
    ("D", KeyCode::KeyD),
    ("E", KeyCode::KeyE),
    ("F", KeyCode::KeyF),
    ("G", KeyCode::KeyG),
    ("H", KeyCode::KeyH),
    ("I", KeyCode::KeyI),
    ("J", KeyCode::KeyJ),
    ("K", KeyCode::KeyK),
    ("L", KeyCode::KeyL),
    ("M", KeyCode::KeyM),
    ("N", KeyCode::KeyN),
    ("O", KeyCode::KeyO),
    ("P", KeyCode::KeyP),
    ("Q", KeyCode::KeyQ),
    ("R", KeyCode::KeyR),
    ("S", KeyCode::KeyS),
    ("T", KeyCode::KeyT),
    ("U", KeyCode::KeyU),
    ("V", KeyCode::KeyV),
    ("W", KeyCode::KeyW),
    ("X", KeyCode::KeyX),
    ("Y", KeyCode::KeyY),
    ("Z", KeyCode::KeyZ),
    ("0", KeyCode::Digit0),
    ("1", KeyCode::Digit1),
    ("2", KeyCode::Digit2),
    ("3", KeyCode::Digit3),
    ("4", KeyCode::Digit4),
    ("5", KeyCode::Digit5),
    ("6", KeyCode::Digit6),
    ("7", KeyCode::Digit7),
    ("8", KeyCode::Digit8),
    ("9", KeyCode::Digit9),
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
    ("F13", KeyCode::F13),
    ("F14", KeyCode::F14),
    ("F15", KeyCode::F15),
    ("F16", KeyCode::F16),
    ("F17", KeyCode::F17),
    ("F18", KeyCode::F18),
    ("F19", KeyCode::F19),
    ("F20", KeyCode::F20),
    ("F21", KeyCode::F21),
    ("F22", KeyCode::F22),
    ("F23", KeyCode::F23),
    ("F24", KeyCode::F24),
    ("Tab", KeyCode::Tab),
    ("Insert", KeyCode::Insert),
    ("Delete", KeyCode::Delete),
    ("Pause", KeyCode::Pause),
    ("ScrollLock", KeyCode::ScrollLock),
    ("PrintScreen", KeyCode::PrintScreen),
    ("Home", KeyCode::Home),
    ("End", KeyCode::End),
    ("PageUp", KeyCode::PageUp),
    ("PageDown", KeyCode::PageDown),
    ("Up", KeyCode::ArrowUp),
    ("Down", KeyCode::ArrowDown),
    ("Left", KeyCode::ArrowLeft),
    ("Right", KeyCode::ArrowRight),
    ("Escape", KeyCode::Escape),
    ("Esc", KeyCode::Escape),
    ("Enter", KeyCode::Enter),
    ("Space", KeyCode::Space),
    ("Backspace", KeyCode::Backspace),
    ("Minus", KeyCode::Minus),
    ("Equal", KeyCode::Equal),
    ("Comma", KeyCode::Comma),
    ("Period", KeyCode::Period),
    ("Slash", KeyCode::Slash),
    ("Semicolon", KeyCode::Semicolon),
    ("Quote", KeyCode::Quote),
    ("Backquote", KeyCode::Backquote),
    ("Backslash", KeyCode::Backslash),
    ("BracketLeft", KeyCode::BracketLeft),
    ("BracketRight", KeyCode::BracketRight),
    ("Numpad0", KeyCode::Numpad0),
    ("Numpad1", KeyCode::Numpad1),
    ("Numpad2", KeyCode::Numpad2),
    ("Numpad3", KeyCode::Numpad3),
    ("Numpad4", KeyCode::Numpad4),
    ("Numpad5", KeyCode::Numpad5),
    ("Numpad6", KeyCode::Numpad6),
    ("Numpad7", KeyCode::Numpad7),
    ("Numpad8", KeyCode::Numpad8),
    ("Numpad9", KeyCode::Numpad9),
    ("NumpadAdd", KeyCode::NumpadAdd),
    ("NumpadSubtract", KeyCode::NumpadSubtract),
    ("NumpadMultiply", KeyCode::NumpadMultiply),
    ("NumpadDivide", KeyCode::NumpadDivide),
    ("NumpadEnter", KeyCode::NumpadEnter),
];

impl KeyChord {
    pub const fn new(key: KeyCode) -> Self {
        Self {
            key,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    /// Parses `F12`, `Ctrl+J` or `Ctrl+Shift+A`: modifiers (`Ctrl` or
    /// `Control`, `Shift`, `Alt`) in any order, then one key name, joined by
    /// `+`, without regard to case. `None` if it names no key.
    pub fn parse(text: &str) -> Option<KeyChord> {
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let name = parts.pop()?;
        let key = KEY_NAMES
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map(|(_, key)| *key)?;
        let mut chord = KeyChord::new(key);
        for part in parts {
            let flag = match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => &mut chord.ctrl,
                "shift" => &mut chord.shift,
                "alt" => &mut chord.alt,
                _ => return None,
            };
            if std::mem::replace(flag, true) {
                return None;
            }
        }
        Some(chord)
    }

    /// Whether the game reads this chord: either mapping function gives it
    /// a byte under its Shift and Ctrl. The mapping ignores Alt, so Alt
    /// plus a reserved key is reserved too.
    pub fn reserved(self) -> bool {
        key_code_to_input_byte(self.key, self.shift, self.ctrl).is_some()
            || key_code_to_line_input_byte(self.key, self.shift, self.ctrl).is_some()
    }
}

/// Why a `[keys]` chord was not bound. Logged at launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyMapNote {
    NotUnderstood {
        key: &'static str,
        chord: String,
    },
    UsedByGame {
        key: &'static str,
        chord: String,
    },
    Taken {
        key: &'static str,
        chord: String,
        by: &'static str,
    },
}

impl fmt::Display for KeyMapNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyMapNote::NotUnderstood { key, chord } => {
                write!(f, "{KEY_NOT_UNDERSTOOD}: {key} = {chord}")
            }
            KeyMapNote::UsedByGame { key, chord } => {
                write!(f, "{KEY_USED_BY_GAME}: {key} = {chord}")
            }
            KeyMapNote::Taken { key, chord, by } => {
                write!(f, "{KEY_TAKEN}: {key} = {chord} ({by})")
            }
        }
    }
}

/// The enabled bindings: no shadow bindings in v1 (U3), no two actions on
/// one chord. Exists only while the router is in the app.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyMap {
    bindings: HashMap<KeyChord, EnhAction>,
    /// The gem map's contextual paging pair is bound (chapter 10).
    gem_paging: bool,
}

impl KeyMap {
    /// Builds the map from `[keys]` entries (name, chord; `None` unbound),
    /// in order, keeping the actions `enabled` accepts. A chord that names
    /// no key, that the game reads, or that an earlier action holds is
    /// refused with a note.
    pub fn build<'n, 'c>(
        entries: impl IntoIterator<Item = (&'n str, Option<&'c str>)>,
        enabled: impl Fn(EnhAction) -> bool,
    ) -> (KeyMap, Vec<KeyMapNote>) {
        let mut map = KeyMap::default();
        let mut notes = Vec::new();
        for (name, chord_text) in entries {
            let Some(action) = EnhAction::from_config_key(name) else {
                continue;
            };
            let (Some(key), Some(chord_text)) = (action.config_key(), chord_text) else {
                continue;
            };
            if !enabled(action) {
                continue;
            }
            let chord_string = chord_text.to_string();
            let Some(chord) = KeyChord::parse(chord_text) else {
                notes.push(KeyMapNote::NotUnderstood {
                    key,
                    chord: chord_string,
                });
                continue;
            };
            if chord.reserved() {
                notes.push(KeyMapNote::UsedByGame {
                    key,
                    chord: chord_string,
                });
                continue;
            }
            if let Some(holder) = map.bindings.get(&chord) {
                notes.push(KeyMapNote::Taken {
                    key,
                    chord: chord_string,
                    by: holder.config_key().unwrap_or_default(),
                });
                continue;
            }
            map.bindings.insert(chord, action);
        }
        (map, notes)
    }

    /// The launch's bindings for the toggles in force.
    pub fn for_toggles(keys: &KeyBindings, toggles: FeatureToggles) -> (KeyMap, Vec<KeyMapNote>) {
        KeyMap::build(keys.iter(), |action| action.enabled(toggles))
    }

    /// Rebuilds the `[keys]` bindings after a toggle change, so a feature
    /// switched off releases its keys in the same frame. The contextual
    /// pair is kept as it is.
    pub fn rebuild(&mut self, keys: &KeyBindings, toggles: FeatureToggles) -> Vec<KeyMapNote> {
        let (mut fresh, notes) = KeyMap::for_toggles(keys, toggles);
        fresh.set_gem_paging(self.gem_paging);
        *self = fresh;
        notes
    }

    /// Whether the gem map's paging pair is bound now.
    pub fn gem_paging(&self) -> bool {
        self.gem_paging
    }

    /// The action bound to `chord`, if any.
    pub fn action(&self, chord: KeyChord) -> Option<EnhAction> {
        self.bindings.get(&chord).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Every binding, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (KeyChord, EnhAction)> + '_ {
        self.bindings
            .iter()
            .map(|(chord, action)| (*chord, *action))
    }

    /// Binds or releases PgUp and PgDn for the gem map's level paging. The
    /// gem map's systems call this as a one-panel dungeon gem map opens and
    /// closes; at that moment the game's only use of any key is to close
    /// the view (10.2.6, D10.12). These are the only bindings on reserved
    /// keys.
    pub fn set_gem_paging(&mut self, open: bool) {
        if self.gem_paging == open {
            return;
        }
        self.gem_paging = open;
        for (key, action) in [
            (KeyCode::PageUp, EnhAction::GemPageUp),
            (KeyCode::PageDown, EnhAction::GemPageDown),
        ] {
            if open {
                self.bindings.insert(KeyChord::new(key), action);
            } else {
                self.bindings.remove(&KeyChord::new(key));
            }
        }
    }
}

/// Sent to feature systems for a hotkey; they never read the keyboard.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnhActionEvent(pub EnhAction);

/// Every key pressed while an overlay is open, for that overlay.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayKey(pub KeyCode);

/// A modal hotkey that broke the order rule and went to the game, which
/// drops it as 1.0.1 does. The toast explains (1.2.6).
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotkeyFellThrough(pub EnhAction);

/// The router's modal flag and key latch.
#[derive(Resource, Debug, Default)]
pub struct KeyLatch {
    /// An overlay is open: every key belongs to it. Set by the overlay
    /// model.
    modal: bool,
    /// `modal` as the router last saw it.
    was_modal: bool,
    /// Keys down, from the event stream.
    down: HashSet<KeyCode>,
    /// Keys held when an overlay opened or closed; ignored until released.
    held: HashSet<KeyCode>,
}

impl KeyLatch {
    /// Opens or ends modal input. The router applies it on its next run.
    pub fn set_modal(&mut self, open: bool) {
        self.modal = open;
    }

    pub fn is_modal(&self) -> bool {
        self.modal
    }

    /// Whether `key` is held over an overlay's opening or closing and is
    /// ignored until released.
    pub fn is_latched(&self, key: KeyCode) -> bool {
        self.held.contains(&key)
    }
}

pub(super) fn is_modifier(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}

/// Whether the features in force want the router even with nothing bound:
/// the menu overlay binds F12 itself (1.3.6).
pub(crate) fn menu_overlay_on(toggles: FeatureToggles) -> bool {
    Toggle::from_key("menu.overlay").is_some_and(|toggle| toggles.on(toggle))
}

/// Adds the router when the launch binds a key, the menu overlay is on, or
/// the gem map's level paging may bind PgUp and PgDn (`gem_paging`:
/// `gem_map.enhanced` and `gem_map.level_paging` on, D-M3.6); otherwise
/// adds nothing. For the gem paging alone the keymap stays empty until a
/// one-panel dungeon gem map opens. Called from
/// [`super::EnhancementsPlugin`].
pub(super) fn add_input_router(
    app: &mut App,
    keys: &KeyBindings,
    toggles: FeatureToggles,
    gem_paging: bool,
) {
    let (keymap, notes) = KeyMap::for_toggles(keys, toggles);
    for note in &notes {
        warn!("{note}");
    }
    if keymap.is_empty() && !menu_overlay_on(toggles) && !gem_paging {
        return;
    }
    install_input_router(app, keymap);
}

/// The router, its resources and its events, with `keymap`.
pub(super) fn install_input_router(app: &mut App, keymap: KeyMap) {
    app.insert_resource(keymap)
        .init_resource::<KeyLatch>()
        .add_event::<EnhActionEvent>()
        .add_event::<OverlayKey>()
        .add_event::<HotkeyFellThrough>()
        .add_systems(PreUpdate, enh_input_router.in_set(EnhSet::Input));
}

/// What one frame's key events say, for the latch and repeat handling.
#[derive(Default)]
struct FrameKeys {
    /// Keys pressed by OS key repeat only.
    repeated: HashSet<KeyCode>,
    /// Keys pressed while latched; they stay away from the game.
    ignored: HashSet<KeyCode>,
    /// The modifiers down when each key of this frame was first pressed,
    /// in event order. A chord typed faster than a frame (Ctrl down, P
    /// down, both up again before the next frame) leaves Ctrl released on
    /// `ButtonInput` by the time the router runs; the event stream still
    /// says Ctrl was down when P went down.
    mods_at_press: HashMap<KeyCode, Mods>,
}

/// Ctrl, Shift and Alt, left or right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Mods {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl Mods {
    fn of_down(down: &HashSet<KeyCode>) -> Self {
        let any = |keys: [KeyCode; 2]| keys.iter().any(|key| down.contains(key));
        Self {
            ctrl: any([KeyCode::ControlLeft, KeyCode::ControlRight]),
            shift: any([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            alt: any([KeyCode::AltLeft, KeyCode::AltRight]),
        }
    }

    fn or(self, other: Self) -> Self {
        Self {
            ctrl: self.ctrl || other.ctrl,
            shift: self.shift || other.shift,
            alt: self.alt || other.alt,
        }
    }
}

/// The hotkey router (1.3.6). See the module notes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn enh_input_router(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    key_events: Option<Res<Events<KeyboardInput>>>,
    focus_events: Option<Res<Events<KeyboardFocusLost>>>,
    mut key_cursor: Local<EventCursor<KeyboardInput>>,
    mut focus_cursor: Local<EventCursor<KeyboardFocusLost>>,
    keymap: Res<KeyMap>,
    mut latch: ResMut<KeyLatch>,
    visual: Option<Res<VisualState>>,
    mut actions: EventWriter<EnhActionEvent>,
    mut overlay_keys: EventWriter<OverlayKey>,
    mut fell_through: EventWriter<HotkeyFellThrough>,
) {
    let latch = &mut *latch;
    // Bevy releases every key when the window loses focus; no release
    // event follows, so the latch lets go too.
    if let Some(events) = focus_events.as_deref()
        && focus_cursor.read(events).count() > 0
    {
        latch.down.clear();
        latch.held.clear();
    }
    // An overlay opened or closed since the last run: what is held now is
    // ignored until released.
    if latch.modal != latch.was_modal {
        latch.held = latch
            .down
            .iter()
            .copied()
            .filter(|key| !is_modifier(*key))
            .collect();
        latch.was_modal = latch.modal;
    }
    let mut frame = FrameKeys::default();
    if let Some(events) = key_events.as_deref() {
        let mut fresh = HashSet::new();
        for event in key_cursor.clone().read(events) {
            let key = event.key_code;
            match event.state {
                ButtonState::Pressed => {
                    frame
                        .mods_at_press
                        .entry(key)
                        .or_insert_with(|| Mods::of_down(&latch.down));
                    latch.down.insert(key);
                    if latch.held.contains(&key) {
                        frame.ignored.insert(key);
                    } else if event.repeat {
                        frame.repeated.insert(key);
                    } else {
                        fresh.insert(key);
                    }
                }
                ButtonState::Released => {
                    latch.down.remove(&key);
                    latch.held.remove(&key);
                }
            }
        }
        frame.repeated.retain(|key| !fresh.contains(key));
    }
    let typed = keys_pressed_in_typed_order(&keyboard, key_events.as_deref(), &mut key_cursor);

    if latch.modal {
        for key in &typed {
            if !is_modifier(*key) && !frame.ignored.contains(key) && !latch.held.contains(key) {
                overlay_keys.write(OverlayKey(*key));
            }
        }
        let all: Vec<KeyCode> = keyboard
            .get_pressed()
            .chain(keyboard.get_just_pressed())
            .chain(keyboard.get_just_released())
            .copied()
            .filter(|key| !is_modifier(*key))
            .collect();
        for key in all {
            keyboard.reset(key);
        }
        return;
    }

    for key in &frame.ignored {
        keyboard.reset(*key);
    }
    if keymap.is_empty() {
        return;
    }
    let held = Mods {
        ctrl: keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
        shift: keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        alt: keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
    };
    // A modifier maps to no byte and is dropped at the read, so it is not
    // a key "ahead" of the hotkey.
    let queue_empty =
        visual.is_none_or(|visual| visual.typed_keys.iter().all(|typed| is_modifier(typed.key)));
    let mut first = true;
    for key in typed {
        if frame.ignored.contains(&key) || is_modifier(key) {
            continue;
        }
        let first_of_frame = std::mem::replace(&mut first, false);
        // The modifiers held now, or down when the key went down this frame.
        let mods = frame
            .mods_at_press
            .get(&key)
            .map_or(held, |at_press| held.or(*at_press));
        let Some(action) = keymap.action(KeyChord {
            key,
            ctrl: mods.ctrl,
            shift: mods.shift,
            alt: mods.alt,
        }) else {
            continue;
        };
        let repeat = frame.repeated.contains(&key);
        if action.is_modal() && !(first_of_frame && queue_empty) {
            // The game gets the key and drops it, as in 1.0.1.
            if !repeat {
                fell_through.write(HotkeyFellThrough(action));
            }
            continue;
        }
        keyboard.reset(key);
        // OS repeat of a held hotkey is consumed but asks for nothing.
        if !repeat {
            actions.write(EnhActionEvent(action));
        }
    }
}

#[cfg(test)]
mod tests;
