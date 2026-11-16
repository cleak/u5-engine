//! The enhancements foundation in the Bevy shell (Phase B design 1.3.5).
//!
//! The host resolves the settings against `u5_runtime`'s catalogue and
//! hands them to the shell in a [`VisualLaunchConfig`]. The shell never
//! reads `config.toml`, the environment or the command line for them.
//!
//! Zero footprint when off is structural:
//!
//! - [`EnhancementsPlugin`] is added only when some toggle is on, and never
//!   under the parity lock ([`VisualLaunchConfig::wants_enhancements`]).
//!   Under Classic no enhancement system, set, edge or resource exists, so
//!   both `Update` chains are the 1.0.1 chains.
//! - Inside it, a feature plugin is added only when
//!   [`feature_wanted`] says so, and every one of its systems carries
//!   `run_if(toggle_on(T))`.
//! - The foundation's own `apply_pending_changes` runs only while
//!   [`PendingToggleChanges`] or [`PendingSettingChanges`] holds
//!   something. Each change applies at its effective point
//!   (`u5_runtime::enhancements::effective_apply`, D-M4.4): the
//!   catalogue's, or the next launch when its plugin was not installed at
//!   launch.
//! - The hotkey router ([`input`]) is added only while some key is bound.
//! - The overlay model ([`overlay`]) is added only while an overlay feature
//!   is on. The idle predicate ([`idle`]) is a run condition and adds
//!   nothing of its own.
//! - The toast line and badges ([`toast`]) and the synthetic-key owner
//!   ([`synthetic`]) are added only by a feature that uses them (the
//!   overlay model brings the toasts for its refusals).
//! - The side-space arbiter and panel host ([`side`]) are added only while
//!   a side-claiming feature is on. Its run condition on the faithful
//!   `fit_display_to_window`, `side::faithful_fit_owns_frame`, is the one
//!   edit to the faithful layout path, and is true while the arbiter is
//!   absent.
//! - The presentation pipeline host ([`present`]) is added only when a
//!   feature registers a step. With no step the frame sprite shows the
//!   faithful image, and no upload site is edited.
//! - The save watcher (`sidecar_io`) is added only while a feature that
//!   keeps a per-save sidecar is on and the host gave a save folder, and
//!   runs only while such a feature is on.
//! - The display chapter's plugin ([`crate::display`]) is added only while
//!   a display toggle or the menu overlay is on.
//! - The gem map's plugin ([`gem_map`]) is added only while
//!   `gem_map.enhanced` is on at launch.
//! - The menu's plugin ([`crate::menu::MenuPlugin`]) is added only while
//!   `menu.overlay` or `menu.title_layer` is on at launch.
//! - The output gain and focus mute ([`crate::menu::audio`]) are added
//!   only by their own conditions (design 3.4.2): the overlay, a volume
//!   below 100 or the player's mute for the gain; focus mute or the
//!   overlay for focus mute.
//! - Automatic combat's plugin ([`auto_combat`]) is added only while
//!   `combat.auto` is on at launch.
//! - The cheats plugin ([`cheats`]) is added only while a cheat or the
//!   watch is effectively on at launch, or the menu overlay is on.
//!
//! The Bevy half of the trajectory-equivalence harness (`equivalence`,
//! test-only) runs the real `--visual` app headless, feature on against
//! Classic, and compares the game frame by frame (design 1.3.12).
//!
//! Every enhancement system sits in one of the six [`EnhSet`]s. The plugin
//! declares their edges once, against the faithful systems of both app
//! chains and the intro builder; feature plugins put their systems in a set
//! and never name a faithful system. The faithful systems themselves are
//! not edited.

pub mod auto_combat;
pub mod cheats;
#[cfg(test)]
pub(crate) mod equivalence;
pub mod gem_map;
pub mod idle;
pub mod input;
pub mod overlay;
pub mod present;
pub mod side;
pub mod sidecar_io;
pub mod synthetic;
pub mod toast;

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use bevy::input::InputSystem;
use bevy::prelude::*;
use u5_runtime::enhancements::{
    ApplyAt, BezelImage, BezelMailbox, FeatureToggles, IdlePromptKind, LockState,
    ResolvedEnhancements, SettingValue, SettingsSink, Toggle, apply_value, dormant_install,
    effective_apply,
};

use self::idle::IdleProbe;
use self::input::KeyMap;
pub use crate::menu::host::MenuHost;
use crate::{
    animate_static_tiles, drive_visual, drive_visual_intro, fit_display_to_window,
    present_camp_apparition_flash, screenshot_system,
};

/// Logged when the host could not store a setting changed in play.
const SETTINGS_NOT_SAVED: &str = "enhancements: a changed setting could not be saved";

/// Everything a visual launch is given by its host (design 1.3.3).
///
/// Chapter 3 adds the menu's slot host, the boot key and the QA menu
/// script here.
#[derive(Clone)]
pub struct VisualLaunchConfig {
    /// The resolved settings: toggles, values, key bindings and the lock.
    pub enhancements: ResolvedEnhancements,
    /// Where settings changed in play are stored. `None`: they last for
    /// this run only.
    pub settings_sink: Option<Arc<dyn SettingsSink>>,
    /// The folder the host chose for the save files. The engine never
    /// derives OS paths itself.
    pub save_dir: Option<PathBuf>,
    /// The folder the host chose for saved change-watch logs,
    /// `<data_base>/watch/` (design 7.2.8). `None`: the watch cannot save.
    pub watch_dir: Option<PathBuf>,
    /// `display.bezel_image`, decoded by the host at launch (design 2.3.3).
    /// `None` when the fill is not a bezel or the file could not be used;
    /// the engine then shows black bars.
    pub bezel: Option<BezelImage>,
    /// Where the host's settings sink posts a bezel chosen while the game
    /// runs (2.3.3). `None`: no sink decodes bezels.
    pub bezel_updates: Option<BezelMailbox>,
    /// The menu's slot store and other host services (design 3.3.1).
    /// `None`: the menu has no slots to offer.
    pub menu_host: Option<Arc<dyn MenuHost>>,
    /// `--boot-key C`: the key queued at boot, which the intro's
    /// pre-flourish poll reads (`systems/intro.md §3` step 6; design
    /// 3.3.5). A player input, not a setting, so it holds under the parity
    /// lock. `None`: the poll sees what it always saw.
    pub boot_key: Option<u8>,
    /// QA only (D-M4.6): keys the menu takes one per frame while it is
    /// open. The host reads them from `U5_MENU_PRESS` and never passes
    /// them under the parity lock; the engine never reads the environment.
    pub menu_script: Option<Vec<char>>,
}

impl VisualLaunchConfig {
    /// The 1.0.1 game: every setting at its default, no sink, no save
    /// folder. The old entry points and every QA suite launch with this.
    pub fn classic() -> Self {
        Self {
            enhancements: ResolvedEnhancements::classic(),
            settings_sink: None,
            save_dir: None,
            watch_dir: None,
            bezel: None,
            bezel_updates: None,
            menu_host: None,
            boot_key: None,
            menu_script: None,
        }
    }

    /// Whether [`EnhancementsPlugin`] belongs in the app: some toggle is
    /// on and the parity lock is off. Valued settings that change anything
    /// map to toggles, so a changed value alone also counts.
    pub fn wants_enhancements(&self) -> bool {
        self.enhancements.lock == LockState::Off && !self.enhancements.is_classic()
    }
}

impl Default for VisualLaunchConfig {
    fn default() -> Self {
        Self::classic()
    }
}

impl fmt::Debug for VisualLaunchConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisualLaunchConfig")
            .field("enhancements", &self.enhancements)
            .field("settings_sink", &self.settings_sink.is_some())
            .field("save_dir", &self.save_dir)
            .field(
                "bezel",
                &self.bezel.as_ref().map(|bezel| (bezel.width, bezel.height)),
            )
            .field("bezel_updates", &self.bezel_updates.is_some())
            .field("menu_host", &self.menu_host.is_some())
            .field("boot_key", &self.boot_key.map(char::from))
            .field("menu_script", &self.menu_script)
            .finish()
    }
}

/// What a visual shell hands back to its host when it ends (design 3.3.2).
/// The default asks for nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShellExit {
    /// `Some`: the host restarts the process after `run()` returns, as a
    /// slot restore in play asks (design 3.3.5 "Relaunch").
    pub relaunch: Option<Relaunch>,
}

/// A restart the shell asks its host for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relaunch {
    /// The new process's `--boot-key`: `Some(b'J')` after a slot restore,
    /// so the restored save loads without the title.
    pub boot_key: Option<u8>,
}

/// Where the running app leaves its [`ShellExit`] for the `_with` entry
/// point to hand back. Inserted only while [`EnhancementsPlugin`] is in the
/// app; under Classic nothing can ask for a relaunch, and the entry points
/// return [`ShellExit::default`].
#[derive(Resource, Clone, Debug, Default)]
pub struct ShellExitSlot(pub Arc<Mutex<Option<ShellExit>>>);

impl ShellExitSlot {
    /// Leaves `exit` for the host. A later call replaces an earlier one.
    pub fn set(&self, exit: ShellExit) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(exit);
    }

    /// What the app left, or the default when it left nothing.
    pub fn take(&self) -> ShellExit {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .unwrap_or_default()
    }
}

/// A fresh slot for one shell run, added to `app` only when `launch`
/// brings the enhancements plugin.
pub(crate) fn add_shell_exit_slot(app: &mut App, launch: &VisualLaunchConfig) -> ShellExitSlot {
    let slot = ShellExitSlot::default();
    if launch.wants_enhancements() {
        app.insert_resource(slot.clone());
    }
    slot
}

/// The six slots every enhancement system runs in (design 1.3.5).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnhSet {
    /// `PreUpdate`, after Bevy's input system: the hotkey router.
    Input,
    /// `Update`, before the input drivers: pending toggle changes and
    /// overlay bookkeeping.
    Apply,
    /// After `Apply`, before the input drivers: synthetic key sources.
    Feed,
    /// After the input drivers, before the idle redraw tick.
    Observe,
    /// After the faithful frame fit.
    Layout,
    /// After `Layout` and the camp apparition flash, before screenshots.
    Present,
}

/// The toggles in force, changed in play only by
/// `apply_pending_changes`. Exists only while [`EnhancementsPlugin`] is
/// in the app.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Toggles(pub FeatureToggles);

/// Run condition: `t` is on. False when there is no [`Toggles`] at all.
pub fn toggle_on(t: Toggle) -> impl Fn(Option<Res<Toggles>>) -> bool + Clone {
    move |toggles| toggles.is_some_and(|x| x.0.on(t))
}

/// Changes requested in play (by the menu), applied at the toggle's
/// effective [`ApplyAt`] point by `apply_pending_changes`.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub struct PendingToggleChanges(pub Vec<(Toggle, bool)>);

/// Valued changes requested in play (by the menu), such as
/// `display.brightness` or `display.scale_mode`, by catalogue key (build
/// plan D-M4.4). Applied by the same rule as [`PendingToggleChanges`] to
/// [`LiveSettings`], which the display chapter's `Display` resource
/// follows, and its `follow_*` systems react; stored through the host's
/// [`SettingsSink`].
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub struct PendingSettingChanges(pub Vec<(&'static str, SettingValue)>);

/// The settings of this run as changes in play leave them. Exists only
/// while [`EnhancementsPlugin`] is in the app.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct LiveSettings {
    /// In force now: the launch's resolution with every change applied in
    /// play since. Its toggles always equal [`Toggles`].
    pub now: ResolvedEnhancements,
    /// What the player chose: `now` with the changes stored for the next
    /// launch too. The menu's rows show these values, each with its apply
    /// point; changes still waiting for the idle prompt are in the queues.
    pub chosen: ResolvedEnhancements,
}

impl LiveSettings {
    /// A run's settings at launch.
    pub fn at_launch(resolved: &ResolvedEnhancements) -> Self {
        Self {
            now: resolved.clone(),
            chosen: resolved.clone(),
        }
    }
}

/// Puts a stored change on a resolution: a toggle's switch, or a value.
fn choose(resolved: &mut ResolvedEnhancements, key: &str, value: &SettingValue) {
    match (u5_runtime::enhancements::CatalogueRow::find(key), value) {
        (Some(u5_runtime::enhancements::CatalogueRow::Toggle(spec)), SettingValue::Bool(on)) => {
            resolved.toggles.set(spec.toggle, *on);
        }
        _ => {
            apply_value(resolved, key, value.clone());
        }
    }
}

/// Queues a change of the setting `key` made in play: a toggle's switch to
/// [`PendingToggleChanges`], any other value to [`PendingSettingChanges`].
/// A flag for a valued toggle (such as `display.brightness`) has no meaning
/// here and is queued as given, to be refused when applied.
pub fn queue_setting_change(
    key: &'static str,
    value: SettingValue,
    toggles: &mut PendingToggleChanges,
    settings: &mut PendingSettingChanges,
) {
    match (u5_runtime::enhancements::CatalogueRow::find(key), &value) {
        (Some(u5_runtime::enhancements::CatalogueRow::Toggle(spec)), SettingValue::Bool(on)) => {
            toggles.0.push((spec.toggle, *on));
        }
        _ => settings.0.push((key, value)),
    }
}

/// The rest of the launch config, for the foundation's own systems and
/// later feature plugins.
#[derive(Resource, Clone, Debug)]
pub struct EnhLaunch(pub VisualLaunchConfig);

/// Whether a feature plugin for `t` is added at launch: its toggle is on,
/// or (the second arm, D-M4.4) the menu overlay is on and `t`'s dormant
/// install is proven inert, so the menu can switch it on later
/// (`u5_runtime::enhancements::DORMANT_WITH_MENU`: the side fill,
/// brightness, compositing and the gem map). Every other feature off at
/// launch applies at the next launch.
pub fn feature_wanted(toggles: FeatureToggles, t: Toggle) -> bool {
    toggles.on(t) || dormant_install(toggles, t)
}

/// The foundation plugin. Add it through [`add_enhancements`], which leaves
/// it out under Classic and under the lock.
pub struct EnhancementsPlugin {
    launch: VisualLaunchConfig,
}

impl EnhancementsPlugin {
    pub fn new(launch: VisualLaunchConfig) -> Self {
        Self { launch }
    }
}

impl Plugin for EnhancementsPlugin {
    fn build(&self, app: &mut App) {
        configure_enh_sets(app);
        app.insert_resource(Toggles(self.launch.enhancements.toggles))
            .init_resource::<PendingToggleChanges>()
            .init_resource::<PendingSettingChanges>()
            .insert_resource(LiveSettings::at_launch(&self.launch.enhancements))
            .insert_resource(EnhLaunch(self.launch.clone()))
            .add_systems(
                Update,
                apply_pending_changes
                    .run_if(changes_pending)
                    .in_set(EnhSet::Apply),
            );
        input::add_input_router(
            app,
            &self.launch.enhancements.keys,
            self.launch.enhancements.toggles,
            gem_map::paging_wanted(&self.launch),
        );
        overlay::add_overlay_model(app, self.launch.enhancements.toggles);
        side::add_side_space(
            app,
            self.launch.enhancements.toggles,
            &self.launch.enhancements.values,
        );
        sidecar_io::add_save_watcher(
            app,
            self.launch.enhancements.toggles,
            self.launch.save_dir.as_deref(),
        );
        crate::menu::add_menu(app, &self.launch);
        crate::menu::audio::add_audio(app, &self.launch);
        crate::menu::start_keys::add_start_keys(app, &self.launch);
        gem_map::add_gem_map(app, &self.launch);
        auto_combat::add_auto_combat(app, &self.launch);
        cheats::add_cheats(app, &self.launch);
        crate::display::add_display(app, &self.launch);
    }
}

/// Adds [`EnhancementsPlugin`] when `launch` wants it; otherwise the app
/// is left exactly as the 1.0.1 builders made it.
pub fn add_enhancements(app: &mut App, launch: &VisualLaunchConfig) {
    if launch.wants_enhancements() {
        app.add_plugins(EnhancementsPlugin::new(launch.clone()));
    }
}

/// The set edges, declared once for both app chains and the intro
/// builder. An edge to a system the app does not register (the intro
/// driver in `--visual`) orders nothing.
fn configure_enh_sets(app: &mut App) {
    app.configure_sets(PreUpdate, EnhSet::Input.after(InputSystem))
        .configure_sets(
            Update,
            (
                EnhSet::Apply
                    .before(drive_visual)
                    .before(drive_visual_intro),
                EnhSet::Feed
                    .after(EnhSet::Apply)
                    .before(drive_visual)
                    .before(drive_visual_intro),
                EnhSet::Observe
                    .after(drive_visual)
                    .after(drive_visual_intro)
                    .before(animate_static_tiles),
                EnhSet::Layout.after(fit_display_to_window),
                EnhSet::Present
                    .after(EnhSet::Layout)
                    .after(present_camp_apparition_flash)
                    .before(screenshot_system),
            ),
        );
}

fn changes_pending(
    toggles: Res<PendingToggleChanges>,
    settings: Res<PendingSettingChanges>,
) -> bool {
    !toggles.0.is_empty() || !settings.0.is_empty()
}

/// Applies pending changes at their effective [`ApplyAt`] point
/// (`effective_apply` for the launch's toggles) and stores the changed
/// keys through the host's sink, toggles first, each queue in request
/// order:
///
/// - `Frame`: applied now.
/// - `IdlePrompt`: held until the game waits at the exploration command
///   prompt with nothing pending (the idle predicate, design 1.3.7).
/// - `NextLaunch`: stored only. This is also the point of a change whose
///   plugin was not installed at launch (grow-left, or scaling from a
///   faithful launch): it changes nothing now.
///
/// A valued change goes to [`LiveSettings`], with the switch of a valued
/// toggle; a refused value is dropped with a warning and stored nowhere.
/// When the toggles changed, the [`KeyMap`] is rebuilt, so a feature
/// switched off releases its keys in the same frame, and the save watcher
/// is told, so a sidecar feature switched on starts from the save on disk.
/// The display chapter's `Display` follows [`LiveSettings`] (its
/// `follow_live_settings`, present while the menu overlay is on).
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_pending_changes(
    mut pending: ResMut<PendingToggleChanges>,
    mut pending_settings: ResMut<PendingSettingChanges>,
    mut live: ResMut<LiveSettings>,
    mut toggles: ResMut<Toggles>,
    launch: Res<EnhLaunch>,
    keymap: Option<ResMut<KeyMap>>,
    save_watch: Option<ResMut<sidecar_io::SaveWatch>>,
    gem_map: Option<ResMut<gem_map::GemMap>>,
    idle: IdleProbe,
) {
    let before = toggles.0;
    let at_idle_prompt = idle.view().at(&[IdlePromptKind::Exploration]);
    let launch_toggles = launch.0.enhancements.toggles;
    let mut now = before;
    let mut stored = drain_pending_toggles(
        &mut pending.0,
        &mut now,
        |t| effective_apply(t.key(), launch_toggles),
        at_idle_prompt,
    );
    let mut next = live.now.clone();
    next.toggles = now;
    stored.extend(drain_pending_settings(
        &mut pending_settings.0,
        &mut next,
        |key| effective_apply(key, launch_toggles),
        at_idle_prompt,
    ));
    if next != live.now {
        live.now = next;
    }
    if !stored.is_empty() {
        let mut chosen = live.chosen.clone();
        for (key, value) in &stored {
            choose(&mut chosen, key, value);
        }
        live.chosen = chosen;
    }
    let after = live.now.toggles;
    if after != before {
        toggles.0 = after;
        if let Some(mut keymap) = keymap {
            for note in keymap.rebuild(&launch.0.enhancements.keys, after) {
                warn!("{note}");
            }
        }
        if let Some(mut save_watch) = save_watch {
            save_watch.toggles_changed(before, after);
        }
        if let Some(mut gem_map) = gem_map {
            gem_map.toggles_changed(before, after);
        }
    }
    if stored.is_empty() {
        return;
    }
    if let Some(sink) = &launch.0.settings_sink
        && let Err(error) = sink.store(&stored)
    {
        warn!("{SETTINGS_NOT_SAVED}: {error}");
    }
}

/// The pure part of [`apply_pending_changes`] for valued changes: applies
/// or holds each change on `live` and returns the settings to store, in
/// request order. A refused value is dropped and stored nowhere.
fn drain_pending_settings(
    pending: &mut Vec<(&'static str, SettingValue)>,
    live: &mut ResolvedEnhancements,
    apply_at: impl Fn(&str) -> ApplyAt,
    at_idle_prompt: bool,
) -> Vec<(String, SettingValue)> {
    let mut stored = Vec::new();
    pending.retain(|(key, value)| {
        let checked = match apply_at(key) {
            ApplyAt::Frame => apply_value(live, key, value.clone()),
            ApplyAt::IdlePrompt if at_idle_prompt => apply_value(live, key, value.clone()),
            ApplyAt::IdlePrompt => return true,
            ApplyAt::NextLaunch => u5_runtime::enhancements::checked_value(key, value.clone()),
        };
        match checked {
            Some(value) => stored.push((key.to_string(), value)),
            None => warn!("enhancements: {key} cannot be set to {value}"),
        }
        false
    });
    stored
}

/// The pure part of [`apply_pending_changes`] for toggles: applies or holds each change
/// and returns the settings to store, in request order. `at_idle_prompt`
/// releases the changes held for the idle prompt.
fn drain_pending_toggles(
    pending: &mut Vec<(Toggle, bool)>,
    toggles: &mut FeatureToggles,
    apply_at: impl Fn(Toggle) -> ApplyAt,
    at_idle_prompt: bool,
) -> Vec<(String, SettingValue)> {
    let mut stored = Vec::new();
    pending.retain(|&(toggle, on)| {
        match apply_at(toggle) {
            ApplyAt::Frame => toggles.set(toggle, on),
            ApplyAt::NextLaunch => {}
            ApplyAt::IdlePrompt if at_idle_prompt => toggles.set(toggle, on),
            ApplyAt::IdlePrompt => return true,
        }
        stored.push((toggle.key().to_string(), SettingValue::Bool(on)));
        false
    });
    stored
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use u5_runtime::enhancements::LockReason;

    use super::*;

    const T: Toggle = Toggle::EnhancementsFreezeOverlays;

    /// A launch with `T` on. The catalogue's only toggle ships later, so
    /// the resolver would never turn it on; the shell does not care.
    fn launch_with_toggle(lock: LockState) -> VisualLaunchConfig {
        let mut launch = VisualLaunchConfig::classic();
        launch.enhancements.toggles.set(T, true);
        launch.enhancements.lock = lock;
        launch
    }

    #[derive(Default)]
    struct RecordingSink(Mutex<Vec<Vec<(String, SettingValue)>>>);

    impl SettingsSink for RecordingSink {
        fn store(&self, changes: &[(String, SettingValue)]) -> Result<(), String> {
            self.0.lock().unwrap().push(changes.to_vec());
            Ok(())
        }
    }

    #[test]
    fn classic_and_locked_launches_want_no_plugin() {
        assert!(!VisualLaunchConfig::classic().wants_enhancements());
        assert!(!VisualLaunchConfig::default().wants_enhancements());
        for reason in [LockReason::CliFlag, LockReason::EnvVar, LockReason::QaEnv] {
            assert!(!launch_with_toggle(LockState::Locked(reason)).wants_enhancements());
        }
        assert!(launch_with_toggle(LockState::Off).wants_enhancements());
    }

    /// Every schedule of `app` with its systems and system sets, sorted.
    fn schedules_outline(app: &App) -> Vec<String> {
        let mut outline: Vec<String> = app
            .world()
            .resource::<bevy::ecs::schedule::Schedules>()
            .iter()
            .flat_map(|(label, schedule)| {
                let graph = schedule.graph();
                let systems = graph
                    .systems()
                    .map(move |(_, system, _)| format!("{label:?} system {}", system.name()));
                let sets = graph
                    .system_sets()
                    .map(move |(_, set, _)| format!("{label:?} set {set:?}"));
                systems.chain(sets).collect::<Vec<_>>()
            })
            .collect();
        outline.sort();
        outline
    }

    #[test]
    fn classic_adds_no_plugin_set_or_resource() {
        let untouched = schedules_outline(&App::new());
        let mut app = App::new();
        add_enhancements(&mut app, &VisualLaunchConfig::classic());
        add_enhancements(
            &mut app,
            &launch_with_toggle(LockState::Locked(LockReason::EnvVar)),
        );
        assert!(!app.is_plugin_added::<EnhancementsPlugin>());
        let world = app.world();
        assert!(!world.contains_resource::<Toggles>());
        assert!(!world.contains_resource::<PendingToggleChanges>());
        assert!(!world.contains_resource::<EnhLaunch>());
        assert_eq!(
            schedules_outline(&app),
            untouched,
            "Classic must schedule nothing"
        );

        let mut on = App::new();
        add_enhancements(&mut on, &launch_with_toggle(LockState::Off));
        assert_ne!(
            schedules_outline(&on),
            untouched,
            "the outline sees the plugin"
        );
    }

    #[test]
    fn a_toggle_on_adds_the_plugin_and_its_resources() {
        let mut app = App::new();
        add_enhancements(&mut app, &launch_with_toggle(LockState::Off));
        assert!(app.is_plugin_added::<EnhancementsPlugin>());
        let world = app.world();
        assert_eq!(
            world.resource::<Toggles>().0.iter_on().collect::<Vec<_>>(),
            [T]
        );
        assert!(world.resource::<PendingToggleChanges>().0.is_empty());
        assert!(world.resource::<EnhLaunch>().0.enhancements.toggles.on(T));
    }

    #[test]
    fn toggle_on_reads_the_toggles_and_is_false_without_them() {
        let mut app = App::new();
        let world = app.world_mut();
        let mut check = IntoSystem::into_system(toggle_on(T));
        check.initialize(world);
        assert!(!check.run((), world), "no Toggles: off");
        world.insert_resource(Toggles(FeatureToggles::CLASSIC));
        assert!(!check.run((), world));
        let mut on = FeatureToggles::CLASSIC;
        on.set(T, true);
        world.insert_resource(Toggles(on));
        assert!(check.run((), world));
    }

    #[test]
    fn pending_changes_apply_at_their_point() {
        let mut toggles = FeatureToggles::CLASSIC;
        let mut pending = vec![(T, true)];
        let stored = drain_pending_toggles(&mut pending, &mut toggles, |_| ApplyAt::Frame, false);
        assert!(toggles.on(T));
        assert!(pending.is_empty());
        assert_eq!(stored, [(T.key().to_string(), SettingValue::Bool(true))]);

        let mut pending = vec![(T, false)];
        let stored =
            drain_pending_toggles(&mut pending, &mut toggles, |_| ApplyAt::NextLaunch, false);
        assert!(toggles.on(T), "a next-launch change is stored only");
        assert!(pending.is_empty());
        assert_eq!(stored, [(T.key().to_string(), SettingValue::Bool(false))]);

        let mut pending = vec![(T, false)];
        let stored =
            drain_pending_toggles(&mut pending, &mut toggles, |_| ApplyAt::IdlePrompt, false);
        assert!(toggles.on(T), "an idle-prompt change waits for the prompt");
        assert_eq!(pending, [(T, false)]);
        assert!(stored.is_empty(), "nothing is stored before it applies");

        let stored =
            drain_pending_toggles(&mut pending, &mut toggles, |_| ApplyAt::IdlePrompt, true);
        assert!(!toggles.on(T), "the idle prompt releases it");
        assert!(pending.is_empty());
        assert_eq!(stored, [(T.key().to_string(), SettingValue::Bool(false))]);
    }

    #[test]
    fn apply_pending_toggles_runs_only_with_changes_and_stores_them() {
        let sink = Arc::new(RecordingSink::default());
        let mut launch = launch_with_toggle(LockState::Off);
        launch.settings_sink = Some(sink.clone());
        let mut app = App::new();
        add_enhancements(&mut app, &launch);

        app.update();
        assert!(sink.0.lock().unwrap().is_empty(), "no change, no store");
        assert!(app.world().resource::<Toggles>().0.on(T));

        // The only catalogue toggle applies at the next frame.
        assert_eq!(T.spec().apply, ApplyAt::Frame);
        app.world_mut()
            .resource_mut::<PendingToggleChanges>()
            .0
            .push((T, false));
        app.update();
        assert!(!app.world().resource::<Toggles>().0.on(T));
        assert!(app.world().resource::<PendingToggleChanges>().0.is_empty());
        assert_eq!(
            *sink.0.lock().unwrap(),
            [vec![(T.key().to_string(), SettingValue::Bool(false))]]
        );
    }

    #[test]
    fn feature_plugins_follow_their_toggle_at_launch() {
        assert!(!feature_wanted(FeatureToggles::CLASSIC, T));
        let mut on = FeatureToggles::CLASSIC;
        on.set(T, true);
        assert!(feature_wanted(on, T));
    }

    /// D-M4.4: the second arm installs only the proven-inert features, and
    /// only with the menu overlay on.
    #[test]
    fn the_second_arm_installs_the_dormant_features_with_the_menu() {
        let mut menu = FeatureToggles::CLASSIC;
        menu.set(Toggle::MenuOverlay, true);
        for spec in u5_runtime::enhancements::TOGGLES {
            let dormant = u5_runtime::enhancements::DORMANT_WITH_MENU.contains(&spec.toggle);
            assert_eq!(
                feature_wanted(menu, spec.toggle),
                dormant || spec.toggle == Toggle::MenuOverlay,
                "{}",
                spec.key
            );
            assert!(
                !feature_wanted(FeatureToggles::CLASSIC, spec.toggle),
                "{}",
                spec.key
            );
        }
    }

    #[test]
    fn pending_settings_apply_at_their_point() {
        let key = "display.brightness";
        let mut live = ResolvedEnhancements::classic();
        let mut pending = vec![(key, SettingValue::Int(70))];
        let stored =
            drain_pending_settings(&mut pending, &mut live, |_| ApplyAt::IdlePrompt, false);
        assert!(stored.is_empty(), "nothing is stored before it applies");
        assert_eq!(pending.len(), 1);
        assert_eq!(live, ResolvedEnhancements::classic());

        let stored = drain_pending_settings(&mut pending, &mut live, |_| ApplyAt::IdlePrompt, true);
        assert!(pending.is_empty());
        assert_eq!(stored, [(key.to_string(), SettingValue::Int(70))]);
        assert_eq!(live.values.int(key), Some(70));
        assert!(live.toggles.on(Toggle::DisplayBrightness));

        let mut pending = vec![(key, SettingValue::Int(85))];
        let stored =
            drain_pending_settings(&mut pending, &mut live, |_| ApplyAt::NextLaunch, false);
        assert_eq!(stored, [(key.to_string(), SettingValue::Int(85))]);
        assert_eq!(live.values.int(key), Some(70), "stored only");

        // A refused value is dropped and stored nowhere.
        let mut pending = vec![(key, SettingValue::Int(3)), (key, SettingValue::Int(100))];
        let stored = drain_pending_settings(&mut pending, &mut live, |_| ApplyAt::Frame, false);
        assert!(pending.is_empty());
        assert_eq!(stored, [(key.to_string(), SettingValue::Int(100))]);
        assert!(!live.toggles.on(Toggle::DisplayBrightness));
    }

    #[test]
    fn a_queued_change_goes_to_its_queue() {
        let mut toggles = PendingToggleChanges::default();
        let mut settings = PendingSettingChanges::default();
        queue_setting_change(
            "display.sprite_composite",
            SettingValue::Bool(true),
            &mut toggles,
            &mut settings,
        );
        queue_setting_change(
            "display.brightness",
            SettingValue::Int(70),
            &mut toggles,
            &mut settings,
        );
        queue_setting_change(
            "display.composite_halo",
            SettingValue::Bool(false),
            &mut toggles,
            &mut settings,
        );
        assert_eq!(toggles.0, [(Toggle::DisplaySpriteComposite, true)]);
        assert_eq!(
            settings.0,
            [
                ("display.brightness", SettingValue::Int(70)),
                ("display.composite_halo", SettingValue::Bool(false)),
            ]
        );
    }
}
