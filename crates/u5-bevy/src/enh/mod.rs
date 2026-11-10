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
//! - The foundation's own `apply_pending_toggles` runs only while
//!   [`PendingToggleChanges`] holds something.
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

#[cfg(test)]
pub(crate) mod equivalence;
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
use std::sync::Arc;

use bevy::input::InputSystem;
use bevy::prelude::*;
use u5_runtime::enhancements::{
    ApplyAt, FeatureToggles, IdlePromptKind, LockState, ResolvedEnhancements, SettingValue,
    SettingsSink, Toggle,
};

use self::idle::IdleProbe;
use self::input::KeyMap;
use crate::{
    animate_static_tiles, drive_visual, drive_visual_intro, fit_display_to_window,
    present_camp_apparition_flash, screenshot_system,
};

/// Logged when the host could not store a setting changed in play.
const SETTINGS_NOT_SAVED: &str = "enhancements: a changed setting could not be saved";

/// Everything a visual launch is given by its host (design 1.3.3).
///
/// Chapter 3 adds the menu's slot host and the boot key here.
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
}

impl VisualLaunchConfig {
    /// The 1.0.1 game: every setting at its default, no sink, no save
    /// folder. The old entry points and every QA suite launch with this.
    pub fn classic() -> Self {
        Self {
            enhancements: ResolvedEnhancements::classic(),
            settings_sink: None,
            save_dir: None,
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
            .finish()
    }
}

/// What a visual shell hands back to its host when it ends. Chapter 3
/// adds the relaunch request; until then a shell exit asks for nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShellExit {}

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
/// `apply_pending_toggles`. Exists only while [`EnhancementsPlugin`] is
/// in the app.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Toggles(pub FeatureToggles);

/// Run condition: `t` is on. False when there is no [`Toggles`] at all.
pub fn toggle_on(t: Toggle) -> impl Fn(Option<Res<Toggles>>) -> bool + Clone {
    move |toggles| toggles.is_some_and(|x| x.0.on(t))
}

/// Changes requested in play (by the menu), applied at the toggle's
/// [`ApplyAt`] point by `apply_pending_toggles`.
#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub struct PendingToggleChanges(pub Vec<(Toggle, bool)>);

/// The rest of the launch config, for the foundation's own systems and
/// later feature plugins.
#[derive(Resource, Clone, Debug)]
pub struct EnhLaunch(pub VisualLaunchConfig);

/// Whether a feature plugin for `t` is added at launch: its toggle is on.
/// Chapter 3's menu overlay adds a second arm, for toggles the menu can
/// switch on later (any toggle not applied at the next launch).
pub fn feature_wanted(toggles: FeatureToggles, t: Toggle) -> bool {
    toggles.on(t)
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
            .insert_resource(EnhLaunch(self.launch.clone()))
            .add_systems(
                Update,
                apply_pending_toggles
                    .run_if(toggle_changes_pending)
                    .in_set(EnhSet::Apply),
            );
        input::add_input_router(
            app,
            &self.launch.enhancements.keys,
            self.launch.enhancements.toggles,
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

fn toggle_changes_pending(pending: Res<PendingToggleChanges>) -> bool {
    !pending.0.is_empty()
}

/// Applies pending toggle changes at their [`ApplyAt`] point and stores
/// the changed keys through the host's sink.
///
/// - `Frame`: applied now.
/// - `IdlePrompt`: held until the game waits at the exploration command
///   prompt with nothing pending (the idle predicate, design 1.3.7).
/// - `NextLaunch`: stored only.
///
/// When the toggles changed, the [`KeyMap`] is rebuilt, so a feature
/// switched off releases its keys in the same frame, and the save watcher
/// is told, so a sidecar feature switched on starts from the save on disk.
fn apply_pending_toggles(
    mut pending: ResMut<PendingToggleChanges>,
    mut toggles: ResMut<Toggles>,
    launch: Res<EnhLaunch>,
    keymap: Option<ResMut<KeyMap>>,
    save_watch: Option<ResMut<sidecar_io::SaveWatch>>,
    idle: IdleProbe,
) {
    let before = toggles.0;
    let at_idle_prompt = idle.view().at(&[IdlePromptKind::Exploration]);
    let stored = drain_pending_toggles(
        &mut pending.0,
        &mut toggles.0,
        |t| t.spec().apply,
        at_idle_prompt,
    );
    if toggles.0 != before
        && let Some(mut keymap) = keymap
    {
        for note in keymap.rebuild(&launch.0.enhancements.keys, toggles.0) {
            warn!("{note}");
        }
    }
    if toggles.0 != before
        && let Some(mut save_watch) = save_watch
    {
        save_watch.toggles_changed(before, toggles.0);
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

/// The pure part of [`apply_pending_toggles`]: applies or holds each change
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
}
