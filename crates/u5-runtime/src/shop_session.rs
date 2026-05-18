//! Unified shop-session wrapper that PlayState can hold while a
//! shop overlay is active. Talk's shop-trigger dispatcher constructs
//! one of these; the input layer routes keystrokes/inputs through the
//! matching per-shop machine and applies the outcome to PlayState's
//! gold/equipment/party counters.

use crate::shop_runtime::*;

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
            Self::Innkeeper(_) => "Innkeeper",
            Self::Reagent(_) => "Herbalist",
            Self::Sage(_) => "Sage",
            Self::Tavern(_) => "Tavern",
            Self::HorseTrader(_) => "Horse Trader",
            Self::ShipBroker(_) => "Shipwright",
            Self::Guild(_) => "Guildmaster",
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
    Some(match dialog_id {
        0x81 => ActiveShopSession::Arms(ArmsShopState::Greeting),
        0x82 => ActiveShopSession::Tavern(TavernState::default()),
        0x83 => ActiveShopSession::HorseTrader(HorseTraderState::Greeting),
        0x84 => ActiveShopSession::ShipBroker(ShipBrokerState::default()),
        0x85 => ActiveShopSession::Reagent(ReagentShopState::default()),
        0x86 => ActiveShopSession::Guild(GuildShopState::default()),
        0x87 => ActiveShopSession::Healer(HealerShopState::Greeting),
        0x88 => ActiveShopSession::Innkeeper(InnkeeperState::default()),
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
}
