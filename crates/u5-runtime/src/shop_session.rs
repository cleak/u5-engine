//! Unified shop-session wrapper that PlayState can hold while a
//! shop overlay is active. Talk's shop-trigger dispatcher constructs
//! one of these; the input layer routes keystrokes/inputs through the
//! matching per-shop machine and applies the outcome to PlayState's
//! gold/equipment/party counters.

use crate::shop_runtime::*;
use crate::shops::{
    ArmsShop, ArmsStockTable, GuildShop, Healer, Herbalist, Inn, Shipwright,
    ShipwrightPurchaseKind, Stable, Tavern, tavern_menu_letters,
};

/// Identifies which of the eight shop kinds is open and owns its
/// per-shop state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActiveShopSession {
    Arms(ArmsShopState),
    ArmsLocal(ArmsShopState, ArmsShop),
    ArmsStocked(ArmsShopState, ArmsStockTable),
    Healer(HealerShopState, Healer),
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
            Self::Arms(s) | Self::ArmsLocal(s, _) | Self::ArmsStocked(s, _) => {
                matches!(s, ArmsShopState::Exited)
            }
            Self::Healer(s, _) => matches!(s, HealerShopState::Exited),
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
            Self::Arms(_) | Self::ArmsStocked(_, _) => "Weaponsmith / Armourer",
            Self::ArmsLocal(_, shop) => shop.display_name(),
            Self::Healer(_, healer) => healer.display_name(),
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
            | Self::Tavern(TavernState::Menu { tavern, .. })
            | Self::Tavern(TavernState::PickProvisionQuantity { tavern, .. })
            | Self::Tavern(TavernState::BlueBoarDrinkList { tavern, .. }) => tavern.display_name(),
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
            Self::ArmsLocal(_, _) | Self::ArmsStocked(_, _) => "Buy (B), Sell (S), or Space.",
            _ => "Choose Buy / Sell / Yes / No.",
        }
    }

    /// State-derived presentation for the active shop overlay.
    ///
    /// This is intentionally semantic rather than a claim of exact
    /// SHOPPE.DAT record selection. The text-window/frontend layers can
    /// present the current shop state consistently while `cleak/u5-spec#62`
    /// tracks exact live bark selection and pacing.
    pub fn modal_summary(&self) -> String {
        let mut lines = vec![format!("{}.", self.modal_shop_label())];
        match self {
            Self::Arms(state) | Self::ArmsLocal(state, _) | Self::ArmsStocked(state, _) => {
                match *state {
                    ArmsShopState::Greeting => lines.push("Buy (B), Sell (S), or Space.".into()),
                    ArmsShopState::BuyPickItem => lines.push("Buy: choose a stock item.".into()),
                    ArmsShopState::BuyConfirm {
                        item, quoted_price, ..
                    } => lines.push(format!("Item {item} costs {quoted_price} gold. Buy? (Y/N)")),
                    ArmsShopState::SellPickItem => {
                        lines.push("Sell: choose an item to sell.".into())
                    }
                    ArmsShopState::SellConfirm { item, offer } => {
                        lines.push(format!("Sell item {item} for {offer} gold? (Y/N)"));
                    }
                    ArmsShopState::Exited => lines.push("Closed.".into()),
                }
            }
            Self::Healer(state, _) => match *state {
                HealerShopState::Greeting => lines.push("Need healing? (Y/N)".into()),
                HealerShopState::PickService => {
                    lines.push("Cure (C), Heal (H), Resurrect (R), or Space.".into());
                }
                HealerShopState::PickPartyMember { service, cost } => {
                    lines.push(format!(
                        "{service:?} costs {cost} gold. Which party member?"
                    ));
                }
                HealerShopState::Confirm {
                    service,
                    slot,
                    cost,
                } => {
                    lines.push(format!(
                        "{service:?} party member {} for {cost} gold? (Y/N)",
                        slot + 1
                    ));
                }
                HealerShopState::Exited => lines.push("Closed.".into()),
            },
            Self::Innkeeper(state) => match *state {
                InnkeeperState::Greeting { .. } => {
                    lines.push("Rest (R), Leave (L), Pick up (P), or Space.".into());
                }
                InnkeeperState::ConfirmRest { total_price, .. } => {
                    lines.push(format!("Rest for {total_price} gold? (Y/N)"));
                }
                InnkeeperState::PickLeaveCompanion { deposit, .. } => {
                    lines.push(format!("Leave companion: deposit is {deposit} gold. Who?"));
                }
                InnkeeperState::ConfirmLeaveCompanion {
                    party_index,
                    deposit,
                    ..
                } => lines.push(format!(
                    "Leave party member {} for {deposit} gold? (Y/N)",
                    party_index + 1
                )),
                InnkeeperState::PickUpCompanion { guest_count, .. } => lines.push(format!(
                    "Pick up companion: choose 1-{guest_count}, or Space."
                )),
                InnkeeperState::ConfirmPickUpCompanion { bill, .. } => {
                    lines.push(format!("Pick up companion for {bill} gold? (Y/N)"));
                }
                InnkeeperState::Exited => lines.push("Closed.".into()),
            },
            Self::Reagent(state) => match *state {
                ReagentShopState::Greeting { .. } | ReagentShopState::PickReagent { .. } => {
                    lines.push("Choose reagent A-E, or Space.".into());
                }
                ReagentShopState::PickQuantity {
                    reagent,
                    unit_price,
                    ..
                } => lines.push(format!(
                    "{} costs {unit_price} gold each. Quantity?",
                    reagent.display_name()
                )),
                ReagentShopState::Exited => lines.push("Closed.".into()),
            },
            Self::Sage(state) => match *state {
                SageState::Prompt { .. } => {
                    lines.push("Of what wouldst thou hear my lore?".into());
                }
                SageState::Confirm { quote, .. } => {
                    lines.push(format!(
                        "{} costs {} gold. Pay? (Y/N)",
                        quote.entry.subject, quote.entry.fee
                    ));
                }
                SageState::Exited => lines.push("Closed.".into()),
            },
            Self::Tavern(state) => match *state {
                TavernState::Greeting { .. } => {
                    lines.push("Drink? Yes (Y), No (N), or Space.".into())
                }
                TavernState::Menu {
                    tavern,
                    continuation_ready,
                } => {
                    let letters = tavern_menu_letters(tavern);
                    let provisions = letters
                        .provisions
                        .map(|letter| format!(", provisions ({letter})"))
                        .unwrap_or_default();
                    let lore_note = if continuation_ready {
                        ""
                    } else {
                        " after a drink"
                    };
                    lines.push(format!(
                        "Round ({}), tavern ({}){provisions}, lore ({}){lore_note}, or Space.",
                        letters.round, letters.secondary, letters.lore
                    ));
                }
                TavernState::PickProvisionQuantity { unit_price, .. } => {
                    lines.push(format!("Provisions cost {unit_price} gold each. Quantity?"));
                }
                TavernState::BlueBoarDrinkList { .. } => {
                    lines.push("Choose Blue Boar drink A-F, or Space.".into());
                }
                TavernState::Exited => lines.push("Closed.".into()),
            },
            Self::HorseTrader(state) => match *state {
                HorseTraderState::Greeting { .. } => lines.push("Buy a horse? (Y/N)".into()),
                HorseTraderState::ConfirmPurchase { price, .. } => {
                    lines.push(format!("Horse costs {price} gold. Buy? (Y/N)"));
                }
                HorseTraderState::Exited => lines.push("Closed.".into()),
            },
            Self::ShipBroker(state) => match *state {
                ShipBrokerState::Greeting { .. } => {
                    lines.push("Choose Frigate (F), Skiff (S), or Space.".into());
                }
                ShipBrokerState::ConfirmPurchase {
                    quote,
                    delivery_x,
                    delivery_y,
                } => {
                    let kind = match quote.kind {
                        ShipwrightPurchaseKind::Frigate => "Frigate",
                        ShipwrightPurchaseKind::Skiff => "Skiff",
                    };
                    lines.push(format!(
                        "{kind} costs {} gold; delivery at ({delivery_x}, {delivery_y}). Buy? (Y/N)",
                        quote.price
                    ));
                }
                ShipBrokerState::Exited => lines.push("Closed.".into()),
            },
            Self::Guild(state) => match *state {
                GuildShopState::Greeting { .. } | GuildShopState::PickItem { .. } => {
                    lines.push("Keys (A), Gems (B), Torches (C), or Space.".into());
                }
                GuildShopState::PickQuantity {
                    commodity,
                    unit_price,
                    ..
                } => lines.push(format!(
                    "{} cost {unit_price} gold each. Quantity?",
                    commodity.display_name()
                )),
                GuildShopState::Exited => lines.push("Closed.".into()),
            },
        }
        lines.join("\n")
    }

    /// Presentation text for a shop modal plus the most recent outcome
    /// message, when present.
    pub fn modal_text(&self, message: &str) -> String {
        let mut text = self.modal_summary();
        if !message.trim().is_empty() {
            text.push('\n');
            text.push_str(message);
        }
        text
    }

    fn modal_shop_label(&self) -> &'static str {
        match self {
            Self::ArmsStocked(_, table) => arms_stock_table_label(*table),
            Self::ArmsLocal(_, shop) => shop.display_name(),
            _ => self.shop_label(),
        }
    }
}

fn arms_stock_table_label(table: ArmsStockTable) -> &'static str {
    const ARMS_SHOPS: [ArmsShop; 9] = [
        ArmsShop::IolosBows,
        ArmsShop::NaughtyNomaans,
        ArmsShop::ArmsOfJustice,
        ArmsShop::DarkwatchArmoury,
        ArmsShop::ThePaladinsProtectorate,
        ArmsShop::NorthStarArmoury,
        ArmsShop::BuccaneersBooty,
        ArmsShop::TheShatteredShield,
        ArmsShop::SiegeCrafters,
    ];

    let mut index = 0usize;
    while index < ARMS_SHOPS.len() {
        let shop = ARMS_SHOPS[index];
        let stock = shop.stock_table();
        if stock.item_ids == table.item_ids && stock.len == table.len {
            return shop.display_name();
        }
        index += 1;
    }
    "Weaponsmith / Armourer"
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
        0x81 => {
            if let Some(scene) = scene_byte {
                let shop = arms_shop_for_scene(scene)?;
                ActiveShopSession::ArmsStocked(ArmsShopState::Greeting, shop.stock_table())
            } else {
                ActiveShopSession::Arms(ArmsShopState::Greeting)
            }
        }
        0x82 => ActiveShopSession::Tavern(match scene_byte {
            Some(scene) => TavernState::for_tavern(tavern_for_scene(scene)?),
            None => TavernState::default(),
        }),
        0x83 => ActiveShopSession::HorseTrader(match scene_byte {
            Some(scene) => HorseTraderState::for_stable(stable_for_scene(scene)?),
            None => HorseTraderState::default(),
        }),
        0x84 => ActiveShopSession::ShipBroker(match scene_byte {
            Some(scene) => ShipBrokerState::for_shipwright(shipwright_for_scene(scene)?),
            None => ShipBrokerState::default(),
        }),
        0x85 => ActiveShopSession::Reagent(match scene_byte {
            Some(scene) => ReagentShopState::for_herbalist(herbalist_for_scene(scene)?),
            None => ReagentShopState::default(),
        }),
        0x86 => ActiveShopSession::Guild(match scene_byte {
            Some(scene) => GuildShopState::for_shop(guild_shop_for_scene(scene)?),
            None => GuildShopState::default(),
        }),
        0x87 => ActiveShopSession::Healer(
            HealerShopState::Greeting,
            match scene_byte {
                Some(scene) => healer_for_scene(scene)?,
                None => Healer::WoundsOfHonour,
            },
        ),
        0x88 => ActiveShopSession::Innkeeper(match scene_byte {
            Some(scene) => InnkeeperState::for_inn(inn_for_scene(scene)?),
            None => InnkeeperState::default(),
        }),
        _ => return None,
    })
}

pub const fn arms_shop_for_scene(scene_byte: u8) -> Option<ArmsShop> {
    Some(match scene_byte {
        2 => ArmsShop::IolosBows,
        3 => ArmsShop::NaughtyNomaans,
        4 => ArmsShop::ArmsOfJustice,
        5 => ArmsShop::DarkwatchArmoury,
        6 => ArmsShop::ThePaladinsProtectorate,
        17 => ArmsShop::NorthStarArmoury,
        24 => ArmsShop::BuccaneersBooty,
        26 => ArmsShop::TheShatteredShield,
        32 => ArmsShop::SiegeCrafters,
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
        22 => Stable::WishingWellHorses,
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

pub const fn healer_for_scene(scene_byte: u8) -> Option<Healer> {
    Some(match scene_byte {
        5 => Healer::TheHealersMission,
        6 => Healer::WoundsOfHonour,
        7 => Healer::TheSpiritHealers,
        21 => Healer::HealersSanctum,
        23 => Healer::Sanctuary,
        30 => Healer::TheShieldOfTruth,
        31 => Healer::TheEmpath,
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
    use crate::shops::{GuildCommodity, Reagent, ShipwrightPurchaseQuote};

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
            Some(ActiveShopSession::Healer(_, _))
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
    fn modal_summary_covers_every_shop_family() {
        let sessions = [
            ActiveShopSession::ArmsStocked(
                ArmsShopState::BuyPickItem,
                ArmsShop::IolosBows.stock_table(),
            ),
            ActiveShopSession::Healer(HealerShopState::PickService, Healer::WoundsOfHonour),
            ActiveShopSession::Innkeeper(InnkeeperState::ConfirmRest {
                inn: Inn::TheWayfarerInn,
                base_room_rate: 2,
                total_price: 12,
            }),
            ActiveShopSession::Reagent(ReagentShopState::PickQuantity {
                herbalist: Herbalist::Mysticism,
                reagent: Reagent::SpiderSilk,
                unit_price: 6,
            }),
            ActiveShopSession::Sage(SageState::default()),
            ActiveShopSession::Tavern(TavernState::Menu {
                tavern: Tavern::TheHonestMeal,
                continuation_ready: true,
            }),
            ActiveShopSession::HorseTrader(HorseTraderState::ConfirmPurchase {
                stable: Stable::HorseAndRider,
                price: 143,
            }),
            ActiveShopSession::ShipBroker(ShipBrokerState::ConfirmPurchase {
                quote: ShipwrightPurchaseQuote {
                    shipwright: Shipwright::TheOakenOar,
                    kind: ShipwrightPurchaseKind::Skiff,
                    price: 125,
                },
                delivery_x: 10,
                delivery_y: 20,
            }),
            ActiveShopSession::Guild(GuildShopState::PickQuantity {
                shop: GuildShop::TheNemesis,
                commodity: GuildCommodity::Keys,
                unit_price: 185,
            }),
        ];

        for session in sessions {
            let text = session.modal_summary();
            assert!(!text.lines().next().unwrap().trim().is_empty(), "{text}");
            assert!(text.lines().count() >= 2, "{text}");
        }
    }

    #[test]
    fn modal_text_appends_last_shop_outcome_message() {
        let session = ActiveShopSession::HorseTrader(HorseTraderState::Greeting {
            stable: Stable::HorseAndRider,
        });
        let text = session.modal_text("Thy horse awaits outside.");

        assert!(text.contains("Horse & Rider"));
        assert!(text.contains("Buy a horse"));
        assert!(text.contains("Thy horse awaits outside."));
    }

    #[test]
    fn talk_context_resolves_scene_local_shop_instances() {
        assert!(matches!(
            shop_session_for_talk_context(0x81, Some(26)),
            Some(ActiveShopSession::ArmsStocked(_, table))
                if table == ArmsShop::TheShatteredShield.stock_table()
        ));
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
            None
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x83, Some(22)),
            Some(ActiveShopSession::HorseTrader(HorseTraderState::Greeting {
                stable: Stable::WishingWellHorses
            }))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x87, Some(5)),
            Some(ActiveShopSession::Healer(
                HealerShopState::Greeting,
                Healer::TheHealersMission
            ))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x87, Some(31)),
            Some(ActiveShopSession::Healer(
                HealerShopState::Greeting,
                Healer::TheEmpath
            ))
        ));
        assert!(matches!(
            shop_session_for_talk_context(0x82, Some(19)),
            Some(ActiveShopSession::Tavern(TavernState::Greeting {
                tavern: Tavern::TheBlueBoarTavern
            }))
        ));
    }

    #[test]
    fn healer_scene_table_matches_published_rows() {
        let cases = [
            (5, Healer::TheHealersMission),
            (6, Healer::WoundsOfHonour),
            (7, Healer::TheSpiritHealers),
            (21, Healer::HealersSanctum),
            (23, Healer::Sanctuary),
            (30, Healer::TheShieldOfTruth),
            (31, Healer::TheEmpath),
        ];
        for (scene, healer) in cases {
            assert_eq!(healer_for_scene(scene), Some(healer), "scene {scene}");
            assert!(matches!(
                shop_session_for_talk_context(0x87, Some(scene)),
                Some(ActiveShopSession::Healer(_, resolved)) if resolved == healer
            ));
        }
        assert_eq!(healer_for_scene(17), None);
        assert!(shop_session_for_talk_context(0x87, Some(17)).is_none());
    }

    #[test]
    fn tavern_scene_table_matches_published_rows() {
        let cases = [
            (1, Tavern::TheHonestMeal),
            (2, Tavern::TheWayfarerTavern),
            (3, Tavern::TheSwordAndKeg),
            (4, Tavern::TheSlaughteredLamb),
            (8, Tavern::TheHumblePalate),
            (19, Tavern::TheBlueBoarTavern),
            (22, Tavern::TheCatsLair),
            (24, Tavern::TheFallenVirgin),
            (30, Tavern::TheFolleyTap),
        ];
        for (scene, tavern) in cases {
            assert_eq!(tavern_for_scene(scene), Some(tavern), "scene {scene}");
            assert!(matches!(
                shop_session_for_talk_context(0x82, Some(scene)),
                Some(ActiveShopSession::Tavern(TavernState::Greeting { tavern: resolved }))
                    if resolved == tavern
            ));
        }
        assert_eq!(tavern_for_scene(6), None);
        assert!(shop_session_for_talk_context(0x82, Some(6)).is_none());
    }

    #[test]
    fn shipwright_scene_table_matches_published_rows() {
        let cases = [
            (3, Shipwright::IslandShipwrights),
            (5, Shipwright::TheCrowsNest),
            (21, Shipwright::TheOakenOar),
            (24, Shipwright::TheRustyBucket),
        ];
        for (scene, shipwright) in cases {
            assert_eq!(
                shipwright_for_scene(scene),
                Some(shipwright),
                "scene {scene}"
            );
            assert!(matches!(
                shop_session_for_talk_context(0x84, Some(scene)),
                Some(ActiveShopSession::ShipBroker(ShipBrokerState::Greeting {
                    shipwright: resolved
                })) if resolved == shipwright
            ));
        }
        assert_eq!(shipwright_for_scene(11), None);
        assert!(shop_session_for_talk_context(0x84, Some(11)).is_none());
    }

    #[test]
    fn reagent_scene_table_matches_published_rows() {
        let cases = [
            (1, Herbalist::TheHerbalist),
            (4, Herbalist::HealersHerbs),
            (7, Herbalist::TheAlchemist),
            (23, Herbalist::Mysticism),
            (30, Herbalist::TheSharperMage),
        ];
        for (scene, herbalist) in cases {
            assert_eq!(herbalist_for_scene(scene), Some(herbalist), "scene {scene}");
            assert!(matches!(
                shop_session_for_talk_context(0x85, Some(scene)),
                Some(ActiveShopSession::Reagent(ReagentShopState::Greeting {
                    herbalist: resolved
                })) if resolved == herbalist
            ));
        }
        assert_eq!(herbalist_for_scene(12), None);
        assert!(shop_session_for_talk_context(0x85, Some(12)).is_none());
    }

    #[test]
    fn guild_scene_table_matches_published_rows() {
        let cases = [
            (8, GuildShop::TheDen),
            (22, GuildShop::TheGuild),
            (24, GuildShop::TheNemesis),
        ];
        for (scene, shop) in cases {
            assert_eq!(guild_shop_for_scene(scene), Some(shop), "scene {scene}");
            assert!(matches!(
                shop_session_for_talk_context(0x86, Some(scene)),
                Some(ActiveShopSession::Guild(GuildShopState::Greeting { shop: resolved }))
                    if resolved == shop
            ));
        }
        assert_eq!(guild_shop_for_scene(19), None);
        assert!(shop_session_for_talk_context(0x86, Some(19)).is_none());
    }

    #[test]
    fn inn_scene_table_matches_published_rows() {
        let cases = [
            (2, Inn::TheWayfarerInn),
            (3, Inn::TheWarriorsStead),
            (7, Inn::TheHauntingInn),
            (20, Inn::HotelBrittany),
            (22, Inn::TheSmugglersInn),
            (24, Inn::TheKingsRansomInn),
        ];
        for (scene, inn) in cases {
            assert_eq!(inn_for_scene(scene), Some(inn), "scene {scene}");
            assert!(matches!(
                shop_session_for_talk_context(0x88, Some(scene)),
                Some(ActiveShopSession::Innkeeper(InnkeeperState::Greeting { inn: resolved }))
                    if resolved == inn
            ));
        }
        assert_eq!(inn_for_scene(19), None);
        assert!(shop_session_for_talk_context(0x88, Some(19)).is_none());
    }

    #[test]
    fn arms_scene_table_matches_published_rows() {
        let cases = [
            (2, ArmsShop::IolosBows, "Iolo's Bows"),
            (3, ArmsShop::NaughtyNomaans, "Naughty Nomaan's"),
            (4, ArmsShop::ArmsOfJustice, "Arms of Justice"),
            (5, ArmsShop::DarkwatchArmoury, "Darkwatch Armoury"),
            (
                6,
                ArmsShop::ThePaladinsProtectorate,
                "The Paladin's Protectorate!",
            ),
            (17, ArmsShop::NorthStarArmoury, "North Star Armoury"),
            (24, ArmsShop::BuccaneersBooty, "Buccaneers Booty"),
            (26, ArmsShop::TheShatteredShield, "The Shattered Shield"),
            (32, ArmsShop::SiegeCrafters, "Siege Crafters"),
        ];
        for (scene, shop, name) in cases {
            assert_eq!(arms_shop_for_scene(scene), Some(shop), "scene {scene}");
            assert_eq!(shop.display_name(), name);
            assert!(matches!(
                shop_session_for_talk_context(0x81, Some(scene)),
                Some(ActiveShopSession::ArmsStocked(_, table)) if table == shop.stock_table()
            ));
        }
        assert_eq!(arms_shop_for_scene(1), None);
        assert!(shop_session_for_talk_context(0x81, Some(1)).is_none());
    }

    #[test]
    fn unknown_scene_specific_shop_returns_none_but_none_context_keeps_default() {
        assert!(shop_session_for_talk_context(0x86, Some(1)).is_none());
        assert!(shop_session_for_talk_context(0x87, Some(1)).is_none());
        assert!(shop_session_for_talk_context(0x83, Some(31)).is_none());
        assert!(shop_session_for_talk_context(0x81, Some(1)).is_none());
        assert!(matches!(
            shop_session_for_talk_context(0x85, None),
            Some(ActiveShopSession::Reagent(ReagentShopState::Greeting {
                herbalist: Herbalist::TheHerbalist
            }))
        ));
    }
}
