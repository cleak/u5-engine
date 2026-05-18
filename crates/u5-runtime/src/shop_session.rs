//! Unified shop-session wrapper that PlayState can hold while a
//! shop overlay is active. Talk's shop-trigger dispatcher constructs
//! one of these; the input layer routes keystrokes/inputs through the
//! matching per-shop machine and applies the outcome to PlayState's
//! gold/equipment/party counters.

use crate::shop_runtime::*;
use crate::shops::{GuildShop, Herbalist, Inn, Shipwright, Stable, Tavern};

/// Identifies which of the eight shop kinds is open and owns its
/// per-shop state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveShopSession {
    Arms(ArmsShopState),
    Healer(HealerShopState),
    Innkeeper(InnkeeperState),
    Reagent(ReagentShopState),
    Sage(SageState),
    Tavern(TavernState),
    HorseTrader(HorseTraderState),
    ShipBroker(ShipBrokerState),
    Guild(GuildShopState),
}

impl ActiveShopSession {
    /// Returns `true` when the underlying machine has reached its
    /// terminal `Exited` state — the caller should drop the session
    /// and return control to the world loop.
    pub fn is_exited(&self) -> bool {
        match self {
            Self::Arms(s) => matches!(s, ArmsShopState::Exited),
            Self::Healer(s) => matches!(s, HealerShopState::Exited),
            Self::Innkeeper(s) => matches!(s, InnkeeperState::Exited),
            Self::Reagent(s) => matches!(s, ReagentShopState::Exited),
            Self::Sage(s) => matches!(s, SageState::Exited),
            Self::Tavern(s) => matches!(s, TavernState::Exited),
            Self::HorseTrader(s) => matches!(s, HorseTraderState::Exited),
            Self::ShipBroker(s) => matches!(s, ShipBrokerState::Exited),
            Self::Guild(s) => matches!(s, GuildShopState::Exited),
        }
    }

    /// Short human-readable label for status/banner display.
    pub fn shop_label(&self) -> &'static str {
        match self {
            Self::Arms(_) => "Weaponsmith / Armourer",
            Self::Healer(_) => "Healer / Sanctum",
            Self::Innkeeper(InnkeeperState::Greeting { inn })
            | Self::Innkeeper(InnkeeperState::ConfirmRest { inn, .. })
            | Self::Innkeeper(InnkeeperState::PickLeaveCompanion { inn, .. })
            | Self::Innkeeper(InnkeeperState::ConfirmLeaveCompanion { inn, .. })
            | Self::Innkeeper(InnkeeperState::PickUpCompanion { inn, .. })
            | Self::Innkeeper(InnkeeperState::ConfirmPickUpCompanion { inn, .. }) => {
                inn.display_name()
            }
            Self::Innkeeper(InnkeeperState::Exited) => "Innkeeper",
            Self::Reagent(ReagentShopState::Greeting { herbalist })
            | Self::Reagent(ReagentShopState::PickReagent { herbalist })
            | Self::Reagent(ReagentShopState::PickQuantity { herbalist, .. }) => {
                herbalist.display_name()
            }
            Self::Reagent(ReagentShopState::Exited) => "Herbalist",
            Self::Sage(_) => "Sage",
            Self::Tavern(TavernState::Greeting { tavern })
            | Self::Tavern(TavernState::Menu { tavern })
            | Self::Tavern(TavernState::PickProvisionQuantity { tavern, .. })
            | Self::Tavern(TavernState::BlueBoarDrinkList { tavern }) => tavern.display_name(),
            Self::Tavern(TavernState::Exited) => "Tavern",
            Self::HorseTrader(HorseTraderState::Greeting { stable })
            | Self::HorseTrader(HorseTraderState::ConfirmPurchase { stable, .. }) => {
                stable.display_name()
            }
            Self::HorseTrader(HorseTraderState::Exited) => "Horse Trader",
            Self::ShipBroker(ShipBrokerState::Greeting { shipwright }) => shipwright.display_name(),
            Self::ShipBroker(ShipBrokerState::ConfirmPurchase { quote, .. }) => {
                quote.shipwright.display_name()
            }
            Self::ShipBroker(ShipBrokerState::Exited) => "Shipwright",
            Self::Guild(GuildShopState::Greeting { shop })
            | Self::Guild(GuildShopState::PickItem { shop })
            | Self::Guild(GuildShopState::PickQuantity { shop, .. }) => shop.display_name(),
            Self::Guild(GuildShopState::Exited) => "Guildmaster",
        }
    }

    /// Short first prompt shown when Talk opens the overlay.
    pub fn opening_prompt(&self) -> &'static str {
        match self {
            Self::Innkeeper(_) => "Rest (R), Leave (L), Pick up (P), or Space.",
            Self::ShipBroker(_) => "Choose Frigate (F), Skiff (S), or Space.",
            Self::Tavern(_) => "Drink? Yes (Y), No (N), or Space.",
            Self::Sage(_) => "Of what wouldst thou hear my lore?",
            Self::Reagent(_) => "Choose reagent A-E, or Space.",
            Self::Guild(_) => "Keys (A), Gems (B), Torches (C), or Space.",
            _ => "Choose Buy / Sell / Yes / No.",
        }
    }
}

/// Build a fresh session for the supplied Talk-resolved shop trigger
/// dialog id. Returns `None` for dialog ids that are not shop
/// triggers per `shops.md §2`.
pub fn shop_session_for_dialog_id(dialog_id: u8) -> Option<ActiveShopSession> {
    shop_session_for_talk_context(dialog_id, None)
}

/// Build a fresh session for a Talk shop trigger, resolving the
/// active scene to the local shop instance where the public shop and
/// scene tables identify one. `scene_byte` uses the town-family scene
/// byte (`1..=32`); `None` falls back to the historic default session
/// for tests and hand-built harnesses.
pub fn shop_session_for_talk_context(
    dialog_id: u8,
    scene_byte: Option<u8>,
) -> Option<ActiveShopSession> {
    Some(match dialog_id {
        0x81 => ActiveShopSession::Arms(ArmsShopState::Greeting),
        0x82 => ActiveShopSession::Tavern(
            scene_byte
                .and_then(tavern_for_scene)
                .map(TavernState::for_tavern)
                .unwrap_or_default(),
        ),
        0x83 => ActiveShopSession::HorseTrader(
            scene_byte
                .and_then(stable_for_scene)
                .map(HorseTraderState::for_stable)
                .unwrap_or_default(),
        ),
        0x84 => ActiveShopSession::ShipBroker(
            scene_byte
                .and_then(shipwright_for_scene)
                .map(ShipBrokerState::for_shipwright)
                .unwrap_or_default(),
        ),
        0x85 => ActiveShopSession::Reagent(
            scene_byte
                .and_then(herbalist_for_scene)
                .map(ReagentShopState::for_herbalist)
                .unwrap_or_default(),
        ),
        0x86 => ActiveShopSession::Guild(
            scene_byte
                .and_then(guild_shop_for_scene)
                .map(GuildShopState::for_shop)
                .unwrap_or_default(),
        ),
        0x87 => ActiveShopSession::Healer(HealerShopState::Greeting),
        0x88 => ActiveShopSession::Innkeeper(
            scene_byte
                .and_then(inn_for_scene)
                .map(InnkeeperState::for_inn)
                .unwrap_or_default(),
        ),
        _ => return None,
    })
}

pub const fn tavern_for_scene(scene_byte: u8) -> Option<Tavern> {
    Some(match scene_byte {
        1 => Tavern::TheHonestMeal,
        2 => Tavern::TheWayfarerTavern,
        3 => Tavern::TheSwordAndKeg,
        4 => Tavern::TheSlaughteredLamb,
        8 => Tavern::TheHumblePalate,
        19 => Tavern::TheBlueBoarTavern,
        22 => Tavern::TheCatsLair,
        24 => Tavern::TheFallenVirgin,
        30 => Tavern::TheFolleyTap,
        _ => return None,
    })
}

pub const fn stable_for_scene(scene_byte: u8) -> Option<Stable> {
    Some(match scene_byte {
        6 => Stable::HorseAndRider,
        20 => Stable::TheStablehouse,
        31 => Stable::WishingWellHorses,
        _ => return None,
    })
}

pub const fn shipwright_for_scene(scene_byte: u8) -> Option<Shipwright> {
    Some(match scene_byte {
        3 => Shipwright::IslandShipwrights,
        5 => Shipwright::TheCrowsNest,
        21 => Shipwright::TheOakenOar,
        24 => Shipwright::TheRustyBucket,
        _ => return None,
    })
}

pub const fn herbalist_for_scene(scene_byte: u8) -> Option<Herbalist> {
    Some(match scene_byte {
        1 => Herbalist::TheHerbalist,
        4 => Herbalist::HealersHerbs,
        7 => Herbalist::TheAlchemist,
        23 => Herbalist::Mysticism,
        30 => Herbalist::TheSharperMage,
        _ => return None,
    })
}

pub const fn guild_shop_for_scene(scene_byte: u8) -> Option<GuildShop> {
    Some(match scene_byte {
        8 => GuildShop::TheDen,
        22 => GuildShop::TheGuild,
        24 => GuildShop::TheNemesis,
        _ => return None,
    })
}

pub const fn inn_for_scene(scene_byte: u8) -> Option<Inn> {
    Some(match scene_byte {
        2 => Inn::TheWayfarerInn,
        3 => Inn::TheWarriorsStead,
        7 => Inn::TheHauntingInn,
        20 => Inn::HotelBrittany,
        22 => Inn::TheSmugglersInn,
        24 => Inn::TheKingsRansomInn,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_id_dispatch_picks_each_shop_kind() {
        assert!(matches!(
            shop_session_for_dialog_id(0x81),
            Some(ActiveShopSession::Arms(_))
        ));
        assert!(matches!(
            shop_session_for_dialog_id(0x82),
            Some(ActiveShopSession::Tavern(_))
        ));
        assert!(matches!(
            shop_session_for_dialog_id(0x83),
            Some(ActiveShopSession::HorseTrader(_))
        ));
        assert!(matches!(
            shop_session_for_dialog_id(0x84),
            Some(ActiveShopSession::ShipBroker(_))
        ));
        assert!(matches!(
            shop_session_for_dialog_id(0x85),
            Some(ActiveShopSession::Reagent(_))
        ));
        assert!(matches!(
            shop_session_for_dialog_id(0x86),
            Some(ActiveShopSession::Guild(_))
        ));
        assert!(matches!(
            shop_session_for_dialog_id(0x87),
            Some(ActiveShopSession::Healer(_))
        ));
        assert!(matches!(
            shop_session_for_dialog_id(0x88),
            Some(ActiveShopSession::Innkeeper(_))
        ));
    }

    #[test]
    fn non_shop_dialog_ids_return_none() {
        assert!(shop_session_for_dialog_id(0x00).is_none());
        assert!(shop_session_for_dialog_id(0x80).is_none());
        assert!(shop_session_for_dialog_id(0x89).is_none());
        assert!(shop_session_for_dialog_id(0xFF).is_none());
    }

    #[test]
    fn fresh_sessions_are_not_yet_exited() {
        for id in 0x81..=0x88 {
            let session = shop_session_for_dialog_id(id).unwrap();
            assert!(!session.is_exited(), "shop 0x{:02X} starts exited", id);
        }
    }

    #[test]
    fn shop_label_returns_non_empty_string_for_each_kind() {
        for id in 0x81..=0x88 {
            let session = shop_session_for_dialog_id(id).unwrap();
            assert!(!session.shop_label().is_empty());
        }
    }

    #[test]
    fn talk_context_resolves_scene_local_shop_instances() {
        assert!(matches!(
            shop_session_for_talk_context(0x86, Some(8)),
            Some(ActiveShopSession::Guild(GuildShopState::Greeting {
                shop: GuildShop::TheDen
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x86, Some(24)),
            Some(ActiveShopSession::Guild(GuildShopState::Greeting {
                shop: GuildShop::TheNemesis
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x85, Some(23)),
            Some(ActiveShopSession::Reagent(ReagentShopState::Greeting {
                herbalist: Herbalist::Mysticism
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x84, Some(21)),
            Some(ActiveShopSession::ShipBroker(ShipBrokerState::Greeting {
                shipwright: Shipwright::TheOakenOar
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x88, Some(20)),
            Some(ActiveShopSession::Innkeeper(InnkeeperState::Greeting {
                inn: Inn::HotelBrittany
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x83, Some(31)),
            Some(ActiveShopSession::HorseTrader(HorseTraderState::Greeting {
                stable: Stable::WishingWellHorses
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x82, Some(19)),
            Some(ActiveShopSession::Tavern(TavernState::Greeting {
                tavern: Tavern::TheBlueBoarTavern
            }))
        ));
    }

    #[test]
    fn unknown_talk_scene_keeps_default_shop_instance() {
        assert!(matches!(
            shop_session_for_talk_context(0x86, Some(1)),
            Some(ActiveShopSession::Guild(GuildShopState::Greeting {
                shop: GuildShop::TheGuild
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x85, None),
            Some(ActiveShopSession::Reagent(ReagentShopState::Greeting {
                herbalist: Herbalist::TheHerbalist
            }))
        ));
    }
}
