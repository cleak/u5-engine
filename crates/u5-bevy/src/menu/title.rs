//! The title layer (Phase B design 3.2.4, D-M4.3; build plan M4-12).
//!
//! Before play the menu key opens the title layer over the title menu.
//! It has its own path, outside the overlay model, which opens only at
//! play prompts:
//!
//! - **Opening.** The router hands `OpenMenu` on only while the title menu
//!   itself polls for a key ([`title_layer_can_open`]); during the flourish,
//!   a story page or a waiting message the key falls through to the intro,
//!   which ignores it as 1.0.1 does, and a real key still skips the
//!   flourish. [`title_actions`] then opens the layer: the page model on
//!   [`Page::Title`](super::pages::Page::Title), the router's modal input
//!   set here ([`KeyLatch::set_modal`]), never [`OverlayState`]
//!   (crate::enh::overlay::OverlayState), and the keys of that frame taken
//!   from the intro, as the overlay model takes them from the game.
//! - **Pages.** Continue, Load, Settings and Quit with slots; Continue,
//!   Settings and Quit without ([`super::pages`]). Settings and Load are
//!   the menu's own pages; Quit leaves the game.
//! - **Continue and Load** restore the slot through the host (the slot
//!   plugin), then type `J` for the player as owner
//!   [`SyntheticOwner::TitleJ`] through the intro's own key step
//!   ([`title_layer_press_journey_onward`]): the game's Journey Onward
//!   runs, with its empty-save message and the emulator-profile load
//!   latency. Without a slot, Continue types the same `J` on the working
//!   save.
//! - **Closing.** F12 or Escape sweeps the layer out and hands the keys
//!   back (held keys stay latched until released). When the game moves on
//!   by itself (a load, the idle Return-to-View preview, which still fires
//!   while the layer is open) the layer goes that frame.
//!
//! The title's own menu and keys are untouched while the layer is shut.

use bevy::prelude::*;
use u5_runtime::FixedCellFont;

use super::pages::{MenuCommand, MenuModel};
use super::render::{MenuView, TitleSweep};
use super::slots::{SlotHost, SlotListing, refresh};
use super::{MenuCommandEvent, MenuState};
use crate::enh::input::{EnhAction, EnhActionEvent, KeyLatch, is_modifier};
use crate::enh::synthetic::{SyntheticInputOwner, SyntheticOwner};
use crate::enh::toast::Toasts;
use crate::{
    VisualIntroState, VisualState, title_layer_can_open, title_layer_press_journey_onward,
};

/// Logged when the title layer cannot read the game's font.
const NO_FONT: &str = "menu: the title layer could not read the game's font, so it stays shut";
/// Logged when another feature holds the synthetic-key token at the title.
const TOKEN_HELD: &str = "menu: another feature is typing, so Continue waits";

/// The game's IBM.CH font, read once when the title layer first opens: the
/// menu in play draws with the game's own copy.
#[derive(Resource, Clone)]
pub struct TitleFont(pub FixedCellFont);

/// Continue or Load at the title restored what it needed: close the layer
/// and type the title menu's `J`.
#[derive(Event, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TitleJourney;

/// Whether the title layer is open.
pub(crate) fn title_open(state: &MenuState) -> bool {
    state.model.as_ref().is_some_and(MenuModel::is_title)
}

/// Opens the title layer when the router hands on the menu key before play
/// and the title menu polls for a key.
#[allow(clippy::too_many_arguments)]
pub(crate) fn title_actions(
    mut commands: Commands,
    mut actions: EventReader<EnhActionEvent>,
    visual: Option<Res<VisualState>>,
    intro: Option<Res<VisualIntroState>>,
    font: Option<Res<TitleFont>>,
    mut state: ResMut<MenuState>,
    mut latch: ResMut<KeyLatch>,
    mut keyboard: Option<ResMut<ButtonInput<KeyCode>>>,
    mut view: ResMut<MenuView>,
    slots: Option<Res<SlotHost>>,
    mut listing: Option<ResMut<SlotListing>>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    let asked = actions
        .read()
        .filter(|action| action.0 == EnhAction::OpenMenu)
        .count()
        > 0;
    let Some(intro) = intro.as_deref() else {
        return;
    };
    if !asked || visual.is_some() || state.model.is_some() || !title_layer_can_open(intro) {
        return;
    }
    if font.is_none() {
        match u5_runtime::load_ibm_ch_font(&intro.game_dir) {
            Ok(font) => commands.insert_resource(TitleFont(font)),
            Err(error) => {
                warn!("{NO_FONT}: {error}");
                return;
            }
        }
    }
    if let (Some(host), Some(listing)) = (slots.as_deref(), listing.as_deref_mut()) {
        refresh(&*host.0, listing, toasts.as_deref_mut());
    }
    state.model = Some(MenuModel::title());
    latch.set_modal(true);
    // The keys typed behind the menu key this frame never reach the intro.
    if let Some(keyboard) = keyboard.as_deref_mut() {
        let keys: Vec<KeyCode> = keyboard
            .get_pressed()
            .chain(keyboard.get_just_pressed())
            .copied()
            .filter(|key| !is_modifier(*key))
            .collect();
        for key in keys {
            keyboard.reset(key);
        }
    }
    view.title_sweep = Some(TitleSweep::Open);
}

/// Shuts the layer: the model goes, the keys go back to the intro, and
/// the picture sweeps out (`sweep`) or goes at once.
fn close_title(
    state: &mut MenuState,
    latch: &mut KeyLatch,
    view: &mut MenuView,
    sweep: TitleSweep,
) {
    state.model = None;
    latch.set_modal(false);
    view.title_sweep = Some(sweep);
}

/// Carries out the title layer's own commands: closing, and Continue's or
/// Load's `J` (3.2.4).
#[allow(clippy::too_many_arguments)]
pub(crate) fn title_commands(
    mut commands: EventReader<MenuCommandEvent>,
    mut journeys: EventReader<TitleJourney>,
    mut state: ResMut<MenuState>,
    mut latch: ResMut<KeyLatch>,
    mut view: ResMut<MenuView>,
    mut intro: Option<ResMut<VisualIntroState>>,
    mut owner: Option<ResMut<SyntheticInputOwner>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut close = false;
    let mut journey = journeys.read().count() > 0;
    for MenuCommandEvent(command) in commands.read() {
        match command {
            MenuCommand::Close => close = true,
            MenuCommand::JourneyOnward => journey = true,
            _ => {}
        }
    }
    if !title_open(&state) {
        return;
    }
    if journey {
        let Some(intro) = intro.as_deref_mut() else {
            return;
        };
        if let Some(owner) = owner.as_deref_mut()
            && owner.take(SyntheticOwner::TitleJ).is_err()
        {
            warn!("{TOKEN_HELD}");
            return;
        }
        // The game takes over the picture at once: no sweep.
        close_title(&mut state, &mut latch, &mut view, TitleSweep::Stop);
        title_layer_press_journey_onward(intro, &mut images);
        if let Some(owner) = owner.as_deref_mut() {
            owner.release(SyntheticOwner::TitleJ);
        }
    } else if close {
        close_title(&mut state, &mut latch, &mut view, TitleSweep::Close);
    }
}

/// Shuts the layer the frame the title menu stops polling for keys: play
/// began, or the idle Return-to-View preview fired (it still does with the
/// layer open, as the keys the layer takes never reach the intro).
pub(crate) fn title_follow_intro(
    visual: Option<Res<VisualState>>,
    intro: Option<Res<VisualIntroState>>,
    mut state: ResMut<MenuState>,
    mut latch: ResMut<KeyLatch>,
    mut view: ResMut<MenuView>,
) {
    if !title_open(&state) {
        return;
    }
    let polling = visual.is_none() && intro.as_deref().is_some_and(title_layer_can_open);
    if !polling {
        close_title(&mut state, &mut latch, &mut view, TitleSweep::Stop);
    }
}

#[cfg(test)]
#[path = "title_tests.rs"]
mod tests;
