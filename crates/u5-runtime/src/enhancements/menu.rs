//! The `[menu]` and `[audio]` settings as the engine sees them (Phase B
//! design 3.3.3, 3.3.6, 3.4.1; build plan M4-02).
//!
//! The rows are in the catalogue ([`super::catalogue`]): the toggles
//! `menu.overlay`, `menu.title_layer`, `menu.named_slots`, `audio.muted`,
//! `audio.mute_on_focus_loss` and the deviation-tier start preferences
//! `audio.start_sound_off` and `audio.start_buffer_on`; the valued
//! `menu.quicksave`, `menu.confirm_unsaved`, `menu.quit_to_title` and
//! `audio.volume`; and the `input.*` toggles of the optional Phase 5. This
//! module turns the resolved values into the plain [`MenuSettings`] and
//! [`AudioSettings`] the shell reads. It is plain data: no serde, no Bevy,
//! no paths.
//!
//! Every row is hidden until its chapter 3 phase ships (U13), and a hidden
//! row has already been resolved to its default, so in a build that ships
//! none of them both views are their defaults. The parity lock gives the
//! defaults too.
//!
//! Nothing here is state-altering. The overlay, slots, title layer and
//! start preferences act only through keys a player could type (class b);
//! volume and the mutes are output gain only (class a, 3.3.7).

use super::catalogue::Toggle;
use super::resolve::ResolvedEnhancements;

pub const OVERLAY_KEY: &str = "menu.overlay";
pub const TITLE_LAYER_KEY: &str = "menu.title_layer";
pub const NAMED_SLOTS_KEY: &str = "menu.named_slots";
pub const QUICKSAVE_KEY: &str = "menu.quicksave";
pub const CONFIRM_UNSAVED_KEY: &str = "menu.confirm_unsaved";
pub const QUIT_TO_TITLE_KEY: &str = "menu.quit_to_title";
pub const VOLUME_KEY: &str = "audio.volume";
pub const MUTED_KEY: &str = "audio.muted";
pub const MUTE_ON_FOCUS_LOSS_KEY: &str = "audio.mute_on_focus_loss";
pub const START_SOUND_OFF_KEY: &str = "audio.start_sound_off";
pub const START_BUFFER_ON_KEY: &str = "audio.start_buffer_on";
pub const GAMEPAD_KEY: &str = "input.gamepad";
pub const MOUSE_KEY: &str = "input.mouse";
pub const EXTRA_BINDINGS_KEY: &str = "input.extra_bindings";

/// The start preferences (3.4.1): deviation tier, in no preset, and each
/// works with every other toggle off.
pub const START_KEYS: &[&str] = &[START_SOUND_OFF_KEY, START_BUFFER_ON_KEY];

/// The `[input]` toggles, which ship with chapter 3's optional Phase 5 and
/// are hidden in every M4 build.
pub const INPUT_KEYS: &[&str] = &[GAMEPAD_KEY, MOUSE_KEY, EXTRA_BINDINGS_KEY];

/// `audio.volume`'s default: the 1.0.1 loudness.
pub const VOLUME_DEFAULT: u8 = 100;
/// The largest `audio.volume`.
pub const VOLUME_MAX: u8 = 100;
/// `audio.volume` moves in steps of this many percent (3.2.7).
pub const VOLUME_STEP: u8 = 5;

/// How Quit to title returns to the title (3.2.5, D3.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum QuitToTitle {
    /// Restart the process: the original's own path, seed included (R1).
    #[default]
    Relaunch,
    /// Rebuild the intro inside the window (R2). Ships with the optional
    /// Phase 7; until then the resolver turns it into [`Relaunch`] with a
    /// note.
    ///
    /// [`Relaunch`]: QuitToTitle::Relaunch
    InApp,
}

impl QuitToTitle {
    /// The word the setting stores.
    pub fn key(self) -> &'static str {
        super::catalogue::QUIT_TO_TITLE_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [QuitToTitle::Relaunch, QuitToTitle::InApp]
            .into_iter()
            .find(|choice| choice.key() == word)
    }
}

/// The typed `[menu]` view (3.3.3). The default is every key at its
/// catalogue default: every toggle off, the quick slot and the unsaved
/// warning on, Quit to title by relaunch.
///
/// `quicksave` matters only with `named_slots` on, and `confirm_unsaved`
/// and `quit_to_title` only with a menu on; this view reports the resolved
/// values as they are and [`quick_slot`](Self::quick_slot) combines them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuSettings {
    /// F12 at the idle exploration prompt.
    pub overlay: bool,
    /// F12 on the title screen.
    pub title_layer: bool,
    pub named_slots: bool,
    pub quicksave: bool,
    pub confirm_unsaved: bool,
    pub quit_to_title: QuitToTitle,
}

impl Default for MenuSettings {
    fn default() -> Self {
        Self {
            overlay: false,
            title_layer: false,
            named_slots: false,
            quicksave: true,
            confirm_unsaved: true,
            quit_to_title: QuitToTitle::Relaunch,
        }
    }
}

impl MenuSettings {
    /// Whether any menu is drawn: the overlay or the title layer. This is
    /// the condition that adds `MenuPlugin` (3.4.2).
    pub fn any_menu(&self) -> bool {
        self.overlay || self.title_layer
    }

    /// Whether the Save and Load pages show the `quick` slot (3.2.2).
    pub fn quick_slot(&self) -> bool {
        self.named_slots && self.quicksave
    }
}

/// The typed `[audio]` view (3.3.3). The default is the 1.0.1 output: full
/// volume, no mute, and no start key typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioSettings {
    /// 0..=100, a multiple of [`VOLUME_STEP`]; 100 is the 1.0.1 loudness.
    pub volume_percent: u8,
    /// The player's own mute.
    pub muted: bool,
    pub mute_on_focus_loss: bool,
    /// Type Ctrl-S once at the first idle prompt.
    pub start_sound_off: bool,
    /// Type Ctrl-B once at the first idle prompt.
    pub start_buffer_on: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume_percent: VOLUME_DEFAULT,
            muted: false,
            mute_on_focus_loss: false,
            start_sound_off: false,
            start_buffer_on: false,
        }
    }
}

impl AudioSettings {
    /// The output gain, `volume_percent / 100`, before any mute (3.3.7).
    pub fn volume(&self) -> f32 {
        f32::from(self.volume_percent) / 100.0
    }

    /// Whether the output gain differs from 1.0.1's at launch: a volume
    /// below 100 or the player's mute. Part of the condition that adds
    /// `VolumePlugin` (3.4.2).
    pub fn gain_changed(&self) -> bool {
        self.volume_percent != VOLUME_DEFAULT || self.muted
    }

    /// Whether a start preference types a key at the first idle prompt,
    /// the condition that adds `StartKeysPlugin` (3.4.2).
    pub fn types_start_keys(&self) -> bool {
        self.start_sound_off || self.start_buffer_on
    }
}

impl ResolvedEnhancements {
    /// The `[menu]` section as [`MenuSettings`] (3.3.3). As for the other
    /// views, a missing or unexpected value reads as the key's default.
    pub fn menu(&self) -> MenuSettings {
        let defaults = MenuSettings::default();
        let values = &self.values;
        MenuSettings {
            overlay: self.toggles.on(Toggle::MenuOverlay),
            title_layer: self.toggles.on(Toggle::MenuTitleLayer),
            named_slots: self.toggles.on(Toggle::MenuNamedSlots),
            quicksave: values.bool(QUICKSAVE_KEY).unwrap_or(defaults.quicksave),
            confirm_unsaved: values
                .bool(CONFIRM_UNSAVED_KEY)
                .unwrap_or(defaults.confirm_unsaved),
            quit_to_title: values
                .str(QUIT_TO_TITLE_KEY)
                .and_then(QuitToTitle::from_key)
                .unwrap_or_default(),
        }
    }

    /// The `[audio]` section as [`AudioSettings`] (3.3.3). A volume outside
    /// 0..=100 is clamped and one off the steps is rounded down to a step;
    /// the resolver already refuses both with a note, so this only guards
    /// a hand-built value.
    pub fn audio(&self) -> AudioSettings {
        let volume_percent = self.values.int(VOLUME_KEY).map_or(VOLUME_DEFAULT, |value| {
            let value = value.clamp(0, i64::from(VOLUME_MAX)) as u8;
            value - value % VOLUME_STEP
        });
        AudioSettings {
            volume_percent,
            muted: self.toggles.on(Toggle::AudioMuted),
            mute_on_focus_loss: self.toggles.on(Toggle::AudioMuteOnFocusLoss),
            start_sound_off: self.toggles.on(Toggle::AudioStartSoundOff),
            start_buffer_on: self.toggles.on(Toggle::AudioStartBufferOn),
        }
    }
}

#[cfg(test)]
mod tests;
