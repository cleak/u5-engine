//! Shop transaction state machines per `systems/shops.md` §8.
//!
//! Each shop kind is its own deterministic-with-inputs state machine:
//! the caller feeds keystrokes or numeric inputs one at a time, the
//! machine emits a transition outcome, and PlayState (or a test
//! harness) applies the gold/HP/stock effects from that outcome.
//!
//! This module covers arms / weaponsmith, healer / sanctum, innkeeper,
//! reagent (herbalist), and tavern shops. Horse trader, ship broker,
//! and guild trader follow the same patterns and reuse the helpers
//! defined here.

use crate::constants::{EQUIPMENT_COUNT, EQUIPMENT_STOCK_CAP};
use crate::shops::{
    ArmsShopAction, ArmsStockTable, BlueBoarDrinkChoice, GuildCommodity, GuildPurchaseError,
    GuildShop, GuildShopAction, Herbalist, INN_REGISTRY_CAP, Inn, InnMainAction,
    ProvisionPurchaseError, Reagent, ReagentPurchaseError, SAGE_RUMOUR_TABLE, SageRumourError,
    SageRumourOutcome, SageRumourQuote, SageRumourTable, Shipwright, ShipwrightMenuAction,
    ShipwrightPurchaseError, ShipwrightPurchaseOutcome, ShipwrightPurchaseQuote, Stable, Tavern,
    TavernDrinkError, TavernDrinkPrompt, apply_blue_boar_drink, apply_guild_purchase,
    apply_provision_purchase, apply_reagent_purchase, apply_shipwright_purchase,
    apply_tavern_round_drink, arms_buy_quote_record_id_for_item, arms_shop_action,
    arms_shop_buy_quote, arms_shop_sell_offer, arms_shop_stock_item_for_letter, guild_shop_action,
    guild_unit_price, herbalist_menu_entries, inn_base_room_rate,
    inn_leave_companion_deposit_for_speaker, inn_main_action, inn_pickup_bill_for_speaker,
    quote_horse_purchase_for_speaker, quote_inn_rest, quote_inn_rest_for_speaker,
    quote_shipwright_purchase, shipwright_delivery_coordinate, shipwright_menu_action,
    tavern_drink_prompt, tavern_lore_menu_letter, tavern_menu_letters, tavern_provision_unit_price,
};
use crate::transport::PendingVehicleAcquisition;

/// Inputs available to every shop machine. Shop-specific machines
/// extract the fields they need.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShopTransactionContext {
    /// Party gold available to spend on Buy / pay / treatment actions.
    pub party_gold: u16,
    /// Speaker's Intelligence byte (per-shop quote weighting).
    pub speaker_intelligence: u8,
    /// In-world hour (used by tavern and innkeeper for time-of-day
    /// pricing where applicable).
    pub world_hour: u8,
    /// Current active party size, used by innkeeper room quotes.
    pub party_size: usize,
    /// Living party members, used by tavern round-drink pricing.
    pub living_party_members: u8,
}

// ---------- Arms shop ----------

/// Arms-shop state machine. Tracks where in the buy/sell branch the
/// player is and produces transition outcomes on input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ArmsShopState {
    /// Initial state: shop has presented Buy/Sell prompt.
    #[default]
    Greeting,
    /// Buy menu open. Player picks an item id `0..EQUIPMENT_COUNT`.
    BuyPickItem,
    /// Shop has quoted a buy price and awaits Yes/No confirmation.
    BuyConfirm {
        item: u8,
        quoted_price: u16,
        quote_record_id: usize,
    },
    /// Sell menu open. Player picks an inventory slot to sell.
    SellPickItem,
    /// Shop has offered a sell price and awaits Yes/No confirmation.
    SellConfirm { item: u8, offer: u16 },
    /// The shop is closed for this turn — no further input is accepted.
    Exited,
}

/// One key press supplied to the arms shop machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmsShopInput {
    /// Raw keystroke (case-insensitive — the machine folds case).
    Key(u8),
    /// Item id from a list pick (Buy or Sell).
    Item(u8),
    /// Buy-menu letter resolved through the current shop's decoded
    /// eight-slot stock table.
    StockLetter { letter: u8, table: ArmsStockTable },
    /// Yes/No confirmation answer.
    Confirm(bool),
}

/// Outcome of one arms-shop transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArmsShopOutcome {
    /// Greeting → entered Buy listing.
    EnteredBuy,
    /// Greeting → entered Sell browser.
    EnteredSell,
    /// Player aborted at any prompt.
    Exited,
    /// Player picked an item to buy; shop quoted a price.
    QuotedBuyPrice {
        item: u8,
        price: u16,
        quote_record_id: usize,
    },
    /// Player picked an item to sell; shop offered a price.
    OfferedSellPrice { item: u8, offer: u16 },
    /// Buy completed: gold debited, item stock incremented.
    Bought { item: u8, paid: u16 },
    /// Sell completed: gold credited, item stock decremented.
    Sold { item: u8, received: u16 },
    /// Buy/Sell declined: state returns to greeting.
    Declined,
    /// Insufficient gold or zero stock to buy — refused without
    /// charging.
    BuyRefusedShortFunds { item: u8, quoted_price: u16 },
    /// Sell refused: no stock of that item.
    SellRefusedNoStock { item: u8 },
    /// Buy refused: shop stock is already at its cap.
    BuyRefusedCapHit { item: u8 },
    /// Input made no semantic sense in the current state — emitted as
    /// a polite no-op.
    InvalidInput,
}

/// Equipment stock view passed to the arms-shop machine.
pub type EquipmentStock = [u8; EQUIPMENT_COUNT];

/// Process one arms-shop input. The caller mutates the state and the
/// shared gold / equipment-stock arrays based on the outcome.
pub fn step_arms_shop(
    state: &mut ArmsShopState,
    input: ArmsShopInput,
    ctx: ShopTransactionContext,
    gold: &mut u16,
    stock: &mut EquipmentStock,
    base_price_table: &[u16; EQUIPMENT_COUNT],
) -> ArmsShopOutcome {
    match (*state, input) {
        (ArmsShopState::Greeting, ArmsShopInput::Key(b)) => match arms_shop_action(b) {
            ArmsShopAction::Buy => {
                *state = ArmsShopState::BuyPickItem;
                ArmsShopOutcome::EnteredBuy
            }
            ArmsShopAction::Sell => {
                *state = ArmsShopState::SellPickItem;
                ArmsShopOutcome::EnteredSell
            }
            ArmsShopAction::Exit => {
                *state = ArmsShopState::Exited;
                ArmsShopOutcome::Exited
            }
        },
        (ArmsShopState::BuyPickItem, ArmsShopInput::Item(item)) => {
            quote_arms_shop_buy_item(state, item, ctx, base_price_table)
        }
        (ArmsShopState::BuyPickItem, ArmsShopInput::StockLetter { letter, table }) => {
            let Ok(item) = arms_shop_stock_item_for_letter(table, letter) else {
                return ArmsShopOutcome::InvalidInput;
            };
            quote_arms_shop_buy_item(state, item, ctx, base_price_table)
        }
        (
            ArmsShopState::BuyConfirm {
                item, quoted_price, ..
            },
            ArmsShopInput::Confirm(true),
        ) => {
            let item_idx = item as usize;
            if stock[item_idx] >= EQUIPMENT_STOCK_CAP {
                *state = ArmsShopState::Greeting;
                return ArmsShopOutcome::BuyRefusedCapHit { item };
            }
            if *gold < quoted_price {
                *state = ArmsShopState::Greeting;
                return ArmsShopOutcome::BuyRefusedShortFunds { item, quoted_price };
            }
            *gold -= quoted_price;
            stock[item_idx] = stock[item_idx].saturating_add(1);
            *state = ArmsShopState::Greeting;
            ArmsShopOutcome::Bought {
                item,
                paid: quoted_price,
            }
        }
        (ArmsShopState::BuyConfirm { .. }, ArmsShopInput::Confirm(false)) => {
            *state = ArmsShopState::Greeting;
            ArmsShopOutcome::Declined
        }
        (ArmsShopState::SellPickItem, ArmsShopInput::Item(item)) => {
            let item_idx = item as usize;
            if item_idx >= EQUIPMENT_COUNT {
                return ArmsShopOutcome::InvalidInput;
            }
            if stock[item_idx] == 0 {
                *state = ArmsShopState::Greeting;
                return ArmsShopOutcome::SellRefusedNoStock { item };
            }
            let base = base_price_table[item_idx];
            let offer = arms_shop_sell_offer(base, ctx.speaker_intelligence);
            *state = ArmsShopState::SellConfirm { item, offer };
            ArmsShopOutcome::OfferedSellPrice { item, offer }
        }
        (ArmsShopState::SellConfirm { item, offer }, ArmsShopInput::Confirm(true)) => {
            let item_idx = item as usize;
            if stock[item_idx] == 0 {
                *state = ArmsShopState::Greeting;
                return ArmsShopOutcome::SellRefusedNoStock { item };
            }
            *gold = gold.saturating_add(offer);
            stock[item_idx] -= 1;
            *state = ArmsShopState::Greeting;
            ArmsShopOutcome::Sold {
                item,
                received: offer,
            }
        }
        (ArmsShopState::SellConfirm { .. }, ArmsShopInput::Confirm(false)) => {
            *state = ArmsShopState::Greeting;
            ArmsShopOutcome::Declined
        }
        (ArmsShopState::Exited, _) => ArmsShopOutcome::Exited,
        _ => ArmsShopOutcome::InvalidInput,
    }
}

fn quote_arms_shop_buy_item(
    state: &mut ArmsShopState,
    item: u8,
    ctx: ShopTransactionContext,
    base_price_table: &[u16; EQUIPMENT_COUNT],
) -> ArmsShopOutcome {
    let item_idx = item as usize;
    if item_idx >= EQUIPMENT_COUNT {
        return ArmsShopOutcome::InvalidInput;
    }
    let base = base_price_table[item_idx];
    if base == 0 {
        // Items the shop does not stock at all are treated as an
        // invalid pick rather than an offer of free goods.
        return ArmsShopOutcome::InvalidInput;
    }
    let Some(quote_record_id) = arms_buy_quote_record_id_for_item(item) else {
        return ArmsShopOutcome::InvalidInput;
    };
    let price = arms_shop_buy_quote(base, ctx.speaker_intelligence);
    *state = ArmsShopState::BuyConfirm {
        item,
        quoted_price: price,
        quote_record_id,
    };
    ArmsShopOutcome::QuotedBuyPrice {
        item,
        price,
        quote_record_id,
    }
}

// ---------- Healer ----------

/// Healer / sanctum services per `shops.md §8.3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerService {
    /// `C` — Cure poison. Flat cost.
    Cure,
    /// `H` — Heal HP. Flat cost; restores party member to max HP.
    Heal,
    /// `R` — Resurrect dead member. Highest cost.
    Resurrect,
}

/// Default healer service costs (gold) used by the standalone
/// healer state machine. The active shop flow uses the scene-local
/// healer table in `shops.rs`; these defaults mirror the
/// `Wounds of Honour` public row from `shops.md §8.3`.
pub const HEALER_COST_CURE: u16 = 25;
pub const HEALER_COST_HEAL: u16 = 40;
pub const HEALER_COST_RESURRECT: u16 = 215;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HealerShopState {
    #[default]
    Greeting,
    PickService,
    PickPartyMember {
        service: HealerService,
        cost: u16,
    },
    Confirm {
        service: HealerService,
        slot: u8,
        cost: u16,
    },
    Exited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerShopInput {
    Key(u8),
    Service(HealerService),
    Slot(u8),
    Confirm(bool),
}

/// Per-member view the healer uses to gate service eligibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealerPartyMemberView {
    pub status: u8, // 'G', 'P', 'D', 'S'
    pub hp: u16,
    pub max_hp: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerOutcome {
    EnteredServiceMenu,
    QuotedCost {
        service: HealerService,
        cost: u16,
    },
    /// Service delivered: gold debited, member's status/HP updated.
    Served {
        service: HealerService,
        slot: u8,
        cost: u16,
    },
    /// Service refused — short on gold.
    RefusedShortFunds {
        cost: u16,
    },
    /// Service refused — invalid target (e.g. cure on Good status).
    RefusedNotEligible {
        service: HealerService,
        slot: u8,
    },
    Declined,
    Exited,
    InvalidInput,
}

pub fn step_healer_shop(
    state: &mut HealerShopState,
    input: HealerShopInput,
    gold: &mut u16,
    members: &mut [HealerPartyMemberView],
) -> HealerOutcome {
    match (*state, input) {
        (HealerShopState::Greeting, HealerShopInput::Key(b)) => match b {
            b'H' | b'h' | b'Y' | b'y' => {
                *state = HealerShopState::PickService;
                HealerOutcome::EnteredServiceMenu
            }
            _ => {
                *state = HealerShopState::Exited;
                HealerOutcome::Exited
            }
        },
        (HealerShopState::PickService, HealerShopInput::Service(service)) => {
            let cost = healer_service_cost(service);
            *state = HealerShopState::PickPartyMember { service, cost };
            HealerOutcome::QuotedCost { service, cost }
        }
        (HealerShopState::PickPartyMember { service, cost }, HealerShopInput::Slot(slot)) => {
            let slot_idx = slot as usize;
            if slot_idx >= members.len() {
                return HealerOutcome::InvalidInput;
            }
            if !healer_service_eligible(service, members[slot_idx]) {
                *state = HealerShopState::Greeting;
                return HealerOutcome::RefusedNotEligible { service, slot };
            }
            *state = HealerShopState::Confirm {
                service,
                slot,
                cost,
            };
            HealerOutcome::QuotedCost { service, cost }
        }
        (
            HealerShopState::Confirm {
                service,
                slot,
                cost,
            },
            HealerShopInput::Confirm(true),
        ) => {
            let slot_idx = slot as usize;
            if *gold < cost {
                *state = HealerShopState::Greeting;
                return HealerOutcome::RefusedShortFunds { cost };
            }
            if !healer_service_eligible(service, members[slot_idx]) {
                *state = HealerShopState::Greeting;
                return HealerOutcome::RefusedNotEligible { service, slot };
            }
            *gold -= cost;
            apply_healer_service(service, &mut members[slot_idx]);
            *state = HealerShopState::Greeting;
            HealerOutcome::Served {
                service,
                slot,
                cost,
            }
        }
        (HealerShopState::Confirm { .. }, HealerShopInput::Confirm(false)) => {
            *state = HealerShopState::Greeting;
            HealerOutcome::Declined
        }
        (HealerShopState::Exited, _) => HealerOutcome::Exited,
        _ => HealerOutcome::InvalidInput,
    }
}

pub const fn healer_service_cost(service: HealerService) -> u16 {
    match service {
        HealerService::Cure => HEALER_COST_CURE,
        HealerService::Heal => HEALER_COST_HEAL,
        HealerService::Resurrect => HEALER_COST_RESURRECT,
    }
}

pub fn healer_service_eligible(service: HealerService, member: HealerPartyMemberView) -> bool {
    match service {
        HealerService::Cure => member.status == b'P',
        HealerService::Heal => member.status != b'D' && member.hp < member.max_hp,
        HealerService::Resurrect => member.status == b'D',
    }
}

pub fn apply_healer_service(service: HealerService, member: &mut HealerPartyMemberView) {
    match service {
        HealerService::Cure => {
            if member.status == b'P' {
                member.status = b'G';
            }
        }
        HealerService::Heal => {
            if member.status != b'D' {
                member.hp = member.max_hp;
            }
        }
        HealerService::Resurrect => {
            if member.status == b'D' {
                member.status = b'G';
                member.hp = member.max_hp.max(1);
            }
        }
    }
}

// ---------- Innkeeper ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InnkeeperState {
    Greeting {
        inn: Inn,
    },
    ConfirmRest {
        inn: Inn,
        base_room_rate: u16,
        total_price: u16,
    },
    PickLeaveCompanion {
        inn: Inn,
        deposit: u16,
    },
    ConfirmLeaveCompanion {
        inn: Inn,
        party_index: usize,
        deposit: u16,
    },
    PickUpCompanion {
        inn: Inn,
        guest_indices: [usize; INN_REGISTRY_CAP],
        guest_count: u8,
        base_lodging_charge: u16,
    },
    ConfirmPickUpCompanion {
        inn: Inn,
        registry_index: usize,
        base_lodging_charge: u16,
        bill: u16,
    },
    Exited,
}

impl Default for InnkeeperState {
    fn default() -> Self {
        Self::Greeting {
            inn: Inn::TheWayfarerInn,
        }
    }
}

impl InnkeeperState {
    pub const fn for_inn(inn: Inn) -> Self {
        Self::Greeting { inn }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InnkeeperInput {
    Key(u8),
    Slot(usize),
    GuestChoice(usize),
    GuestChoiceWithStay { choice: usize, stay_counter: u8 },
    Confirm(bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InnkeeperOutcome {
    QuotedRest {
        inn: Inn,
        base_room_rate: u16,
        total_price: u16,
    },
    RestConfirmed {
        inn: Inn,
        base_room_rate: u16,
        total_price: u16,
    },
    PickLeaveCompanion {
        deposit: u16,
    },
    QuotedLeaveCompanion {
        party_index: usize,
        deposit: u16,
    },
    LeaveConfirmed {
        party_index: usize,
        deposit: u16,
    },
    PickUpCompanion,
    QuotedPickUpCompanion {
        registry_index: usize,
        bill: u16,
    },
    PickUpConfirmed {
        registry_index: usize,
        bill: u16,
    },
    Declined,
    Exited,
    InvalidInput,
}

pub fn step_innkeeper(
    state: &mut InnkeeperState,
    input: InnkeeperInput,
    ctx: ShopTransactionContext,
) -> InnkeeperOutcome {
    match (*state, input) {
        (InnkeeperState::Greeting { inn }, InnkeeperInput::Key(b)) => match inn_main_action(b) {
            InnMainAction::Rest => {
                let quote =
                    quote_inn_rest_for_speaker(inn, ctx.party_size, ctx.speaker_intelligence)
                        .or_else(|_| quote_inn_rest(inn, ctx.party_size, inn_base_room_rate(inn)));
                let base_room_rate = inn_base_room_rate(inn);
                let total_price = quote.map(|quote| quote.total_price).unwrap_or(0);
                *state = InnkeeperState::ConfirmRest {
                    inn,
                    base_room_rate,
                    total_price,
                };
                InnkeeperOutcome::QuotedRest {
                    inn,
                    base_room_rate,
                    total_price,
                }
            }
            InnMainAction::LeaveCompanion => {
                let deposit =
                    inn_leave_companion_deposit_for_speaker(inn, ctx.speaker_intelligence);
                *state = InnkeeperState::PickLeaveCompanion { inn, deposit };
                InnkeeperOutcome::PickLeaveCompanion { deposit }
            }
            InnMainAction::PickUpCompanion => {
                *state = InnkeeperState::PickUpCompanion {
                    inn,
                    guest_indices: [0; INN_REGISTRY_CAP],
                    guest_count: 0,
                    base_lodging_charge: inn_base_room_rate(inn),
                };
                InnkeeperOutcome::PickUpCompanion
            }
            InnMainAction::Exit => {
                *state = InnkeeperState::Exited;
                InnkeeperOutcome::Exited
            }
            InnMainAction::Discard => InnkeeperOutcome::InvalidInput,
        },
        (
            InnkeeperState::ConfirmRest {
                inn,
                base_room_rate,
                total_price,
            },
            InnkeeperInput::Confirm(true),
        ) => {
            *state = InnkeeperState::Greeting { inn };
            InnkeeperOutcome::RestConfirmed {
                inn,
                base_room_rate,
                total_price,
            }
        }
        (InnkeeperState::ConfirmRest { inn, .. }, InnkeeperInput::Confirm(false)) => {
            *state = InnkeeperState::Greeting { inn };
            InnkeeperOutcome::Declined
        }
        (
            InnkeeperState::PickLeaveCompanion { inn, deposit },
            InnkeeperInput::Slot(party_index),
        ) => {
            *state = InnkeeperState::ConfirmLeaveCompanion {
                inn,
                party_index,
                deposit,
            };
            InnkeeperOutcome::QuotedLeaveCompanion {
                party_index,
                deposit,
            }
        }
        (
            InnkeeperState::ConfirmLeaveCompanion {
                inn,
                party_index,
                deposit,
            },
            InnkeeperInput::Confirm(true),
        ) => {
            *state = InnkeeperState::Greeting { inn };
            InnkeeperOutcome::LeaveConfirmed {
                party_index,
                deposit,
            }
        }
        (InnkeeperState::ConfirmLeaveCompanion { inn, .. }, InnkeeperInput::Confirm(false)) => {
            *state = InnkeeperState::Greeting { inn };
            InnkeeperOutcome::Declined
        }
        (
            InnkeeperState::PickUpCompanion {
                inn,
                guest_indices,
                guest_count,
                base_lodging_charge,
            },
            InnkeeperInput::GuestChoice(choice),
        ) if choice < guest_count as usize => {
            let registry_index = guest_indices[choice];
            let _ = base_lodging_charge;
            let bill = inn_pickup_bill_for_speaker(inn, 1, ctx.speaker_intelligence);
            *state = InnkeeperState::ConfirmPickUpCompanion {
                inn,
                registry_index,
                base_lodging_charge,
                bill,
            };
            InnkeeperOutcome::QuotedPickUpCompanion {
                registry_index,
                bill,
            }
        }
        (
            InnkeeperState::PickUpCompanion {
                inn,
                guest_indices,
                guest_count,
                base_lodging_charge,
            },
            InnkeeperInput::GuestChoiceWithStay {
                choice,
                stay_counter,
            },
        ) if choice < guest_count as usize => {
            let registry_index = guest_indices[choice];
            let _ = base_lodging_charge;
            let bill = inn_pickup_bill_for_speaker(inn, stay_counter, ctx.speaker_intelligence);
            *state = InnkeeperState::ConfirmPickUpCompanion {
                inn,
                registry_index,
                base_lodging_charge,
                bill,
            };
            InnkeeperOutcome::QuotedPickUpCompanion {
                registry_index,
                bill,
            }
        }
        (
            InnkeeperState::ConfirmPickUpCompanion {
                inn,
                registry_index,
                base_lodging_charge: _,
                bill,
            },
            InnkeeperInput::Confirm(true),
        ) => {
            *state = InnkeeperState::Greeting { inn };
            InnkeeperOutcome::PickUpConfirmed {
                registry_index,
                bill,
            }
        }
        (InnkeeperState::ConfirmPickUpCompanion { inn, .. }, InnkeeperInput::Confirm(false)) => {
            *state = InnkeeperState::Greeting { inn };
            InnkeeperOutcome::Declined
        }
        (InnkeeperState::Exited, _) => InnkeeperOutcome::Exited,
        _ => InnkeeperOutcome::InvalidInput,
    }
}

// ---------- Reagent shop ----------

/// Reagent shop transaction. The player picks a compact letter entry
/// from the current herbalist's stocked menu and then a quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReagentShopState {
    Greeting {
        herbalist: Herbalist,
    },
    PickReagent {
        herbalist: Herbalist,
    },
    PickQuantity {
        herbalist: Herbalist,
        reagent: Reagent,
        unit_price: u16,
    },
    Exited,
}

impl Default for ReagentShopState {
    fn default() -> Self {
        Self::Greeting {
            herbalist: Herbalist::TheHerbalist,
        }
    }
}

impl ReagentShopState {
    pub const fn for_herbalist(herbalist: Herbalist) -> Self {
        Self::Greeting { herbalist }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReagentShopInput {
    Key(u8),
    Quantity(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReagentShopOutcome {
    EnteredMenu {
        herbalist: Herbalist,
    },
    QuotedUnit {
        herbalist: Herbalist,
        reagent: Reagent,
        unit_price: u16,
    },
    Bought {
        herbalist: Herbalist,
        reagent: Reagent,
        quantity: u8,
        paid: u16,
    },
    RefusedShortFunds {
        cost: u16,
    },
    RefusedStockCap {
        current: u8,
        requested: u8,
        cap: u8,
    },
    Declined,
    Exited,
    InvalidInput,
}

pub const REAGENT_STOCK_CAP_PER_KIND: u8 = 99;

pub fn step_reagent_shop(
    state: &mut ReagentShopState,
    input: ReagentShopInput,
    gold: &mut u16,
    reagent_stock: &mut [u8],
) -> ReagentShopOutcome {
    match (*state, input) {
        (ReagentShopState::Greeting { herbalist }, ReagentShopInput::Key(b)) => {
            *state = ReagentShopState::PickReagent { herbalist };
            select_reagent_menu_entry(state, herbalist, b)
        }
        (ReagentShopState::PickReagent { herbalist }, ReagentShopInput::Key(b)) => {
            select_reagent_menu_entry(state, herbalist, b)
        }
        (
            ReagentShopState::PickQuantity {
                herbalist, reagent, ..
            },
            ReagentShopInput::Quantity(quantity),
        ) => {
            *state = ReagentShopState::PickReagent { herbalist };
            if quantity == 0 {
                return ReagentShopOutcome::Declined;
            }
            let idx = reagent.inventory_index();
            let Some(stock) = reagent_stock.get_mut(idx) else {
                return ReagentShopOutcome::InvalidInput;
            };
            match apply_reagent_purchase(gold, stock, herbalist, reagent, quantity) {
                Ok(outcome) => ReagentShopOutcome::Bought {
                    herbalist,
                    reagent,
                    quantity,
                    paid: outcome.quote.total_price,
                },
                Err(ReagentPurchaseError::InsufficientGold { required, .. }) => {
                    ReagentShopOutcome::RefusedShortFunds { cost: required }
                }
                Err(ReagentPurchaseError::StockCap {
                    current,
                    requested,
                    cap,
                }) => ReagentShopOutcome::RefusedStockCap {
                    current,
                    requested,
                    cap,
                },
                Err(ReagentPurchaseError::ZeroQuantity) => ReagentShopOutcome::Declined,
                Err(ReagentPurchaseError::NotStocked) => ReagentShopOutcome::InvalidInput,
            }
        }
        (ReagentShopState::Exited, _) => ReagentShopOutcome::Exited,
        _ => ReagentShopOutcome::InvalidInput,
    }
}

fn select_reagent_menu_entry(
    state: &mut ReagentShopState,
    herbalist: Herbalist,
    byte: u8,
) -> ReagentShopOutcome {
    match byte {
        b' ' | 0x1B | b'N' | b'n' => {
            *state = ReagentShopState::Exited;
            ReagentShopOutcome::Exited
        }
        b'Y' | b'y' => ReagentShopOutcome::EnteredMenu { herbalist },
        b => {
            let upper = b.to_ascii_uppercase() as char;
            let Some(entry) = herbalist_menu_entries(herbalist)
                .into_iter()
                .find(|entry| entry.letter == upper)
            else {
                return ReagentShopOutcome::InvalidInput;
            };
            *state = ReagentShopState::PickQuantity {
                herbalist,
                reagent: entry.reagent,
                unit_price: entry.unit_price,
            };
            ReagentShopOutcome::QuotedUnit {
                herbalist,
                reagent: entry.reagent,
                unit_price: entry.unit_price,
            }
        }
    }
}

// ---------- Tavern ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TavernState {
    Greeting {
        tavern: Tavern,
    },
    Menu {
        tavern: Tavern,
        continuation_ready: bool,
    },
    PickProvisionQuantity {
        tavern: Tavern,
        unit_price: u16,
        continuation_ready: bool,
    },
    BlueBoarDrinkList {
        tavern: Tavern,
        continuation_ready: bool,
    },
    Exited,
}

impl Default for TavernState {
    fn default() -> Self {
        Self::Greeting {
            tavern: Tavern::TheHonestMeal,
        }
    }
}

impl TavernState {
    pub const fn for_tavern(tavern: Tavern) -> Self {
        Self::Greeting { tavern }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TavernInput {
    Key(u8),
    Quantity(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TavernOutcome {
    EnteredMenu {
        tavern: Tavern,
        round_letter: char,
        secondary_letter: char,
        provisions_letter: Option<char>,
        lore_letter: char,
    },
    EnteredSagePrompt,
    RoundDrinkServed {
        tavern: Tavern,
        cost: u16,
    },
    SecondaryTavernSelected {
        tavern: Tavern,
        letter: char,
    },
    PickBlueBoarDrink,
    BlueBoarDrinkServed {
        choice: BlueBoarDrinkChoice,
        cost: u16,
    },
    PickProvisionQuantity {
        tavern: Tavern,
        unit_price: u16,
    },
    ProvisionsPurchased {
        tavern: Tavern,
        requested_quantity: u16,
        purchased_quantity: u16,
        paid: u16,
        food_added: u16,
    },
    Declined,
    RefusedShortFunds {
        cost: u16,
    },
    RefusedNoLivingParty,
    RefusedNoNeed,
    Exited,
    InvalidInput,
}

pub const fn tavern_sage_lore_menu_letter(tavern: Tavern) -> char {
    tavern_lore_menu_letter(tavern)
}

pub fn step_tavern(
    state: &mut TavernState,
    input: TavernInput,
    ctx: ShopTransactionContext,
    gold: &mut u16,
    food: &mut u16,
) -> TavernOutcome {
    match (*state, input) {
        (TavernState::Greeting { tavern }, TavernInput::Key(b)) => match tavern_drink_prompt(b) {
            TavernDrinkPrompt::Enter => {
                let letters = tavern_menu_letters(tavern);
                *state = TavernState::Menu {
                    tavern,
                    continuation_ready: false,
                };
                TavernOutcome::EnteredMenu {
                    tavern,
                    round_letter: letters.round,
                    secondary_letter: letters.secondary,
                    provisions_letter: letters.provisions,
                    lore_letter: letters.lore,
                }
            }
            TavernDrinkPrompt::Leave => {
                *state = TavernState::Exited;
                TavernOutcome::Exited
            }
            TavernDrinkPrompt::Discard => TavernOutcome::InvalidInput,
        },
        (
            TavernState::Menu {
                tavern,
                continuation_ready,
            },
            TavernInput::Key(b),
        ) => {
            let upper = b.to_ascii_uppercase();
            if upper == b' ' || upper == 0x1B || upper == b'N' {
                *state = TavernState::Exited;
                return TavernOutcome::Exited;
            }
            let letters = tavern_menu_letters(tavern);
            if upper == letters.round as u8 {
                let outcome = apply_tavern_round_drink(gold, tavern, ctx.living_party_members);
                return match outcome {
                    Ok(outcome) => {
                        *state = TavernState::Menu {
                            tavern,
                            continuation_ready: true,
                        };
                        TavernOutcome::RoundDrinkServed {
                            tavern,
                            cost: outcome.total_price,
                        }
                    }
                    Err(TavernDrinkError::NoLivingParty) => TavernOutcome::RefusedNoLivingParty,
                    Err(TavernDrinkError::InsufficientGold { required, .. }) => {
                        TavernOutcome::RefusedShortFunds { cost: required }
                    }
                };
            }
            if upper == letters.secondary as u8 {
                if matches!(tavern, Tavern::TheBlueBoarTavern) {
                    *state = TavernState::BlueBoarDrinkList {
                        tavern,
                        continuation_ready,
                    };
                    return TavernOutcome::PickBlueBoarDrink;
                }
                *state = TavernState::Menu {
                    tavern,
                    continuation_ready: true,
                };
                return TavernOutcome::SecondaryTavernSelected {
                    tavern,
                    letter: letters.secondary,
                };
            }
            if letters
                .provisions
                .is_some_and(|letter| upper == letter as u8)
            {
                let unit_price = tavern_provision_unit_price(tavern);
                *state = TavernState::PickProvisionQuantity {
                    tavern,
                    unit_price,
                    continuation_ready,
                };
                return TavernOutcome::PickProvisionQuantity { tavern, unit_price };
            }
            if upper == letters.lore as u8 {
                if !continuation_ready {
                    return TavernOutcome::InvalidInput;
                }
                *state = TavernState::Exited;
                return TavernOutcome::EnteredSagePrompt;
            }
            TavernOutcome::InvalidInput
        }
        (
            TavernState::PickProvisionQuantity {
                tavern,
                continuation_ready,
                ..
            },
            TavernInput::Quantity(quantity),
        ) => {
            let outcome = apply_provision_purchase(gold, food, tavern, quantity);
            match outcome {
                Ok(outcome) => {
                    *state = TavernState::Menu {
                        tavern,
                        continuation_ready: outcome.purchased_quantity > 0,
                    };
                    TavernOutcome::ProvisionsPurchased {
                        tavern,
                        requested_quantity: outcome.requested_quantity,
                        purchased_quantity: outcome.purchased_quantity,
                        paid: outcome.total_price,
                        food_added: outcome.food_after.saturating_sub(outcome.food_before),
                    }
                }
                Err(ProvisionPurchaseError::ZeroQuantity) => {
                    *state = TavernState::Menu {
                        tavern,
                        continuation_ready,
                    };
                    TavernOutcome::Declined
                }
                Err(ProvisionPurchaseError::NoNeed) => {
                    *state = TavernState::Menu {
                        tavern,
                        continuation_ready,
                    };
                    TavernOutcome::RefusedNoNeed
                }
                Err(ProvisionPurchaseError::InsufficientGold {
                    required_per_unit, ..
                }) => {
                    *state = TavernState::Menu {
                        tavern,
                        continuation_ready,
                    };
                    TavernOutcome::RefusedShortFunds {
                        cost: required_per_unit,
                    }
                }
            }
        }
        (
            TavernState::BlueBoarDrinkList {
                tavern,
                continuation_ready,
            },
            TavernInput::Key(b),
        ) => {
            let Some(choice) = blue_boar_choice_for_key(b) else {
                return TavernOutcome::InvalidInput;
            };
            let outcome = apply_blue_boar_drink(gold, choice);
            match outcome {
                Ok(outcome) => {
                    *state = TavernState::Menu {
                        tavern,
                        continuation_ready: true,
                    };
                    TavernOutcome::BlueBoarDrinkServed {
                        choice,
                        cost: outcome.total_price,
                    }
                }
                Err(TavernDrinkError::NoLivingParty) => TavernOutcome::RefusedNoLivingParty,
                Err(TavernDrinkError::InsufficientGold { required, .. }) => {
                    *state = TavernState::Menu {
                        tavern,
                        continuation_ready,
                    };
                    TavernOutcome::RefusedShortFunds { cost: required }
                }
            }
        }
        (TavernState::Exited, _) => TavernOutcome::Exited,
        _ => TavernOutcome::InvalidInput,
    }
}

pub const fn blue_boar_choice_for_key(byte: u8) -> Option<BlueBoarDrinkChoice> {
    match byte {
        b'A' | b'a' => Some(BlueBoarDrinkChoice::A),
        b'B' | b'b' => Some(BlueBoarDrinkChoice::B),
        b'C' | b'c' => Some(BlueBoarDrinkChoice::C),
        b'D' | b'd' => Some(BlueBoarDrinkChoice::D),
        b'E' | b'e' => Some(BlueBoarDrinkChoice::E),
        b'F' | b'f' => Some(BlueBoarDrinkChoice::F),
        _ => None,
    }
}

// ---------- Sage / rumour vendor ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SageState {
    Prompt {
        table: &'static SageRumourTable,
    },
    Confirm {
        table: &'static SageRumourTable,
        quote: SageRumourQuote,
    },
    Exited,
}

impl Default for SageState {
    fn default() -> Self {
        Self::Prompt {
            table: &SAGE_RUMOUR_TABLE,
        }
    }
}

impl SageState {
    pub const fn for_table(table: &'static SageRumourTable) -> Self {
        Self::Prompt { table }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SageInput<'a> {
    Keyword(&'a str),
    Confirm { accepted: bool, record_id: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SageOutcome {
    QuotedFee {
        quote: SageRumourQuote,
    },
    RumourFound {
        outcome: SageRumourOutcome,
        rendered: String,
    },
    Declined,
    RefusedShortFunds {
        required: u16,
        available: u16,
    },
    InputTooLong {
        limit: usize,
        actual: usize,
    },
    NoTopicMatch,
    Exited,
    InvalidInput,
}

pub fn step_sage(state: &mut SageState, input: SageInput<'_>, gold: &mut u16) -> SageOutcome {
    match (*state, input) {
        (SageState::Prompt { table }, SageInput::Keyword(text)) => {
            match crate::shops::find_sage_topic(table, text) {
                Ok(quote) => {
                    *state = SageState::Confirm { table, quote };
                    SageOutcome::QuotedFee { quote }
                }
                Err(SageRumourError::EmptyInput) => {
                    *state = SageState::Exited;
                    SageOutcome::Exited
                }
                Err(SageRumourError::InputTooLong { limit, actual }) => {
                    SageOutcome::InputTooLong { limit, actual }
                }
                Err(SageRumourError::NoTopicMatch) => SageOutcome::NoTopicMatch,
            }
        }
        (
            SageState::Confirm { .. },
            SageInput::Confirm {
                accepted: false, ..
            },
        ) => {
            *state = SageState::Exited;
            SageOutcome::Declined
        }
        (
            SageState::Confirm { table: _, quote },
            SageInput::Confirm {
                accepted: true,
                record_id,
            },
        ) => {
            if *gold < quote.entry.fee {
                *state = SageState::Exited;
                return SageOutcome::RefusedShortFunds {
                    required: quote.entry.fee,
                    available: *gold,
                };
            }
            *gold -= quote.entry.fee;
            *state = SageState::Exited;
            let outcome = SageRumourOutcome {
                quote,
                record_id,
                rendered: crate::shops::render_sage_rumour(quote.entry, record_id),
            };
            let rendered = outcome.rendered.clone();
            SageOutcome::RumourFound { outcome, rendered }
        }
        (SageState::Exited, _) => SageOutcome::Exited,
        _ => SageOutcome::InvalidInput,
    }
}

// ---------- Horse trader ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorseTraderState {
    Greeting { stable: Stable },
    ConfirmPurchase { stable: Stable, price: u16 },
    Exited,
}

impl Default for HorseTraderState {
    fn default() -> Self {
        Self::Greeting {
            stable: Stable::HorseAndRider,
        }
    }
}

impl HorseTraderState {
    pub const fn for_stable(stable: Stable) -> Self {
        Self::Greeting { stable }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorseTraderInput {
    Key {
        key: u8,
        speaker_intelligence: u8,
    },
    Confirm {
        accepted: bool,
        can_place_horse: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorseTraderOutcome {
    QuotedPrice { price: u16 },
    Purchased { price: u16 },
    RefusedShortFunds { price: u16 },
    RefusedNoMarker { price: u16 },
    Declined,
    Exited,
    InvalidInput,
}

pub fn step_horse_trader(
    state: &mut HorseTraderState,
    input: HorseTraderInput,
    gold: &mut u16,
) -> HorseTraderOutcome {
    match (*state, input) {
        (
            HorseTraderState::Greeting { stable },
            HorseTraderInput::Key {
                key,
                speaker_intelligence,
            },
        ) => match key {
            b'Y' | b'y' | b'B' | b'b' => {
                let price = quote_horse_purchase_for_speaker(stable, speaker_intelligence).price;
                *state = HorseTraderState::ConfirmPurchase { stable, price };
                HorseTraderOutcome::QuotedPrice { price }
            }
            _ => {
                *state = HorseTraderState::Exited;
                HorseTraderOutcome::Exited
            }
        },
        (
            HorseTraderState::ConfirmPurchase { stable, price },
            HorseTraderInput::Confirm {
                accepted: true,
                can_place_horse,
            },
        ) => {
            if *gold < price {
                *state = HorseTraderState::Greeting { stable };
                return HorseTraderOutcome::RefusedShortFunds { price };
            }
            if !can_place_horse {
                *state = HorseTraderState::Greeting { stable };
                return HorseTraderOutcome::RefusedNoMarker { price };
            }
            *gold -= price;
            *state = HorseTraderState::Exited;
            HorseTraderOutcome::Purchased { price }
        }
        (
            HorseTraderState::ConfirmPurchase { stable, .. },
            HorseTraderInput::Confirm {
                accepted: false, ..
            },
        ) => {
            *state = HorseTraderState::Greeting { stable };
            HorseTraderOutcome::Declined
        }
        (HorseTraderState::Exited, _) => HorseTraderOutcome::Exited,
        _ => HorseTraderOutcome::InvalidInput,
    }
}

// ---------- Ship broker / shipwright ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipBrokerState {
    Greeting {
        shipwright: Shipwright,
    },
    ConfirmPurchase {
        quote: ShipwrightPurchaseQuote,
        delivery_x: usize,
        delivery_y: usize,
    },
    Exited,
}

impl Default for ShipBrokerState {
    fn default() -> Self {
        Self::Greeting {
            shipwright: Shipwright::IslandShipwrights,
        }
    }
}

impl ShipBrokerState {
    pub const fn for_shipwright(shipwright: Shipwright) -> Self {
        Self::Greeting { shipwright }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipBrokerInput {
    Key(u8),
    Confirm(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipBrokerOutcome {
    QuotedPurchase { quote: ShipwrightPurchaseQuote },
    PurchaseApplied { outcome: ShipwrightPurchaseOutcome },
    RefusedShortFunds { available: u16, required: u16 },
    Declined,
    Exited,
    InvalidInput,
}

pub fn step_ship_broker(
    state: &mut ShipBrokerState,
    input: ShipBrokerInput,
    gold: &mut u16,
    pending_vehicle: &mut Option<PendingVehicleAcquisition>,
) -> ShipBrokerOutcome {
    match (*state, input) {
        (ShipBrokerState::Greeting { shipwright }, ShipBrokerInput::Key(b)) => {
            match shipwright_menu_action(b) {
                ShipwrightMenuAction::Purchase(kind) => {
                    let quote = quote_shipwright_purchase(shipwright, kind);
                    let (delivery_x, delivery_y) = shipwright_delivery_coordinate(shipwright);
                    *state = ShipBrokerState::ConfirmPurchase {
                        quote,
                        delivery_x,
                        delivery_y,
                    };
                    ShipBrokerOutcome::QuotedPurchase { quote }
                }
                ShipwrightMenuAction::Exit => {
                    *state = ShipBrokerState::Exited;
                    ShipBrokerOutcome::Exited
                }
                ShipwrightMenuAction::Discard => ShipBrokerOutcome::InvalidInput,
            }
        }
        (
            ShipBrokerState::ConfirmPurchase {
                quote,
                delivery_x,
                delivery_y,
            },
            ShipBrokerInput::Confirm(true),
        ) => {
            let shipwright = quote.shipwright;
            let outcome = apply_shipwright_purchase(
                gold,
                pending_vehicle,
                quote.shipwright,
                quote.kind,
                delivery_x,
                delivery_y,
            );
            *state = ShipBrokerState::Greeting { shipwright };
            match outcome {
                Ok(outcome) => ShipBrokerOutcome::PurchaseApplied { outcome },
                Err(ShipwrightPurchaseError::InsufficientGold {
                    available,
                    required,
                }) => ShipBrokerOutcome::RefusedShortFunds {
                    available,
                    required,
                },
                Err(ShipwrightPurchaseError::NoReturnWorld) => ShipBrokerOutcome::InvalidInput,
            }
        }
        (ShipBrokerState::ConfirmPurchase { quote, .. }, ShipBrokerInput::Confirm(false)) => {
            *state = ShipBrokerState::Greeting {
                shipwright: quote.shipwright,
            };
            ShipBrokerOutcome::Declined
        }
        (ShipBrokerState::Exited, _) => ShipBrokerOutcome::Exited,
        _ => ShipBrokerOutcome::InvalidInput,
    }
}

// ---------- Guild trader ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildShopState {
    Greeting {
        shop: GuildShop,
    },
    PickItem {
        shop: GuildShop,
    },
    PickQuantity {
        shop: GuildShop,
        commodity: GuildCommodity,
        unit_price: u16,
    },
    Exited,
}

impl Default for GuildShopState {
    fn default() -> Self {
        Self::Greeting {
            shop: GuildShop::TheGuild,
        }
    }
}

impl GuildShopState {
    pub const fn for_shop(shop: GuildShop) -> Self {
        Self::Greeting { shop }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildShopInput {
    Key(u8),
    Quantity(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildShopOutcome {
    EnteredMenu {
        shop: GuildShop,
    },
    QuotedUnit {
        shop: GuildShop,
        commodity: GuildCommodity,
        unit_price: u16,
    },
    Bought {
        shop: GuildShop,
        commodity: GuildCommodity,
        quantity: u8,
        paid: u16,
    },
    RefusedShortFunds {
        cost: u16,
    },
    RefusedStockCap {
        current: u8,
        requested: u8,
        cap: u8,
    },
    Declined,
    Exited,
    InvalidInput,
}

pub fn step_guild_shop(
    state: &mut GuildShopState,
    input: GuildShopInput,
    gold: &mut u16,
    gems: &mut u8,
    keys: &mut u8,
    torches: &mut u8,
) -> GuildShopOutcome {
    match (*state, input) {
        (GuildShopState::Greeting { shop }, GuildShopInput::Key(b)) => {
            *state = GuildShopState::PickItem { shop };
            select_guild_menu_entry(state, shop, b)
        }
        (GuildShopState::PickItem { shop }, GuildShopInput::Key(b)) => {
            select_guild_menu_entry(state, shop, b)
        }
        (
            GuildShopState::PickQuantity {
                shop, commodity, ..
            },
            GuildShopInput::Quantity(quantity),
        ) => {
            *state = GuildShopState::PickItem { shop };
            if quantity == 0 {
                return GuildShopOutcome::Declined;
            }
            let stock = match commodity {
                GuildCommodity::Gems => gems,
                GuildCommodity::Keys => keys,
                GuildCommodity::Torches => torches,
            };
            match apply_guild_purchase(gold, stock, shop, commodity, quantity) {
                Ok(outcome) => GuildShopOutcome::Bought {
                    shop,
                    commodity,
                    quantity,
                    paid: outcome.quote.total_price,
                },
                Err(GuildPurchaseError::InsufficientGold { required, .. }) => {
                    GuildShopOutcome::RefusedShortFunds { cost: required }
                }
                Err(GuildPurchaseError::StockCap {
                    current,
                    requested,
                    cap,
                }) => GuildShopOutcome::RefusedStockCap {
                    current,
                    requested,
                    cap,
                },
                Err(GuildPurchaseError::ZeroQuantity) => GuildShopOutcome::Declined,
            }
        }
        (GuildShopState::Exited, _) => GuildShopOutcome::Exited,
        _ => GuildShopOutcome::InvalidInput,
    }
}

fn select_guild_menu_entry(
    state: &mut GuildShopState,
    shop: GuildShop,
    byte: u8,
) -> GuildShopOutcome {
    match byte {
        b'Y' | b'y' => GuildShopOutcome::EnteredMenu { shop },
        b => match guild_shop_action(b) {
            GuildShopAction::Purchase(commodity) => {
                let unit_price = guild_unit_price(shop, commodity);
                *state = GuildShopState::PickQuantity {
                    shop,
                    commodity,
                    unit_price,
                };
                GuildShopOutcome::QuotedUnit {
                    shop,
                    commodity,
                    unit_price,
                }
            }
            GuildShopAction::Exit => {
                *state = GuildShopState::Exited;
                GuildShopOutcome::Exited
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shops::{
        SAGE_RUMOUR_SUCCESS_RECORD_FIRST, SAGE_RUMOUR_TABLE, SageRumourOutcome, SageRumourQuote,
    };

    fn make_price_table() -> [u16; EQUIPMENT_COUNT] {
        let mut table = [0u16; EQUIPMENT_COUNT];
        // First 10 slots: cheap weapons; next 10: armour; etc.
        for (i, p) in table.iter_mut().enumerate() {
            *p = ((i + 1) * 5) as u16;
        }
        table
    }

    fn make_stock() -> EquipmentStock {
        [0u8; EQUIPMENT_COUNT]
    }

    #[test]
    fn arms_shop_buy_path_debits_gold_and_increments_stock() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let mut stock = make_stock();
        let mut gold = 100u16;
        let ctx = ShopTransactionContext {
            party_gold: gold,
            speaker_intelligence: 10,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };

        assert_eq!(
            step_arms_shop(
                &mut state,
                ArmsShopInput::Key(b'B'),
                ctx,
                &mut gold,
                &mut stock,
                &prices,
            ),
            ArmsShopOutcome::EnteredBuy
        );

        let outcome = step_arms_shop(
            &mut state,
            ArmsShopInput::Item(3),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        assert!(matches!(
            outcome,
            ArmsShopOutcome::QuotedBuyPrice { item: 3, .. }
        ));

        let outcome = step_arms_shop(
            &mut state,
            ArmsShopInput::Confirm(true),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        assert!(matches!(outcome, ArmsShopOutcome::Bought { item: 3, .. }));
        assert_eq!(stock[3], 1);
        assert!(gold < 100);
    }

    #[test]
    fn arms_shop_buy_letter_uses_decoded_stock_table_without_item_translation() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let table = ArmsStockTable::new([11, 7, 23, 0, 0, 0, 0, 0], 3);
        let mut stock = make_stock();
        let mut gold = 1000u16;
        let ctx = ShopTransactionContext {
            speaker_intelligence: 10,
            ..ShopTransactionContext::default()
        };

        assert_eq!(
            step_arms_shop(
                &mut state,
                ArmsShopInput::Key(b'B'),
                ctx,
                &mut gold,
                &mut stock,
                &prices,
            ),
            ArmsShopOutcome::EnteredBuy
        );

        assert_eq!(
            step_arms_shop(
                &mut state,
                ArmsShopInput::StockLetter {
                    letter: b'B',
                    table,
                },
                ctx,
                &mut gold,
                &mut stock,
                &prices,
            ),
            ArmsShopOutcome::QuotedBuyPrice {
                item: 7,
                price: arms_shop_buy_quote(prices[7], 10),
                quote_record_id: arms_buy_quote_record_id_for_item(7).unwrap(),
            }
        );

        assert_eq!(
            step_arms_shop(
                &mut state,
                ArmsShopInput::Confirm(true),
                ctx,
                &mut gold,
                &mut stock,
                &prices,
            ),
            ArmsShopOutcome::Bought {
                item: 7,
                paid: arms_shop_buy_quote(prices[7], 10),
            }
        );
        assert_eq!(stock[7], 1);
        assert_eq!(stock[1], 0);
    }

    #[test]
    fn arms_shop_buy_letter_rejects_letters_outside_decoded_stock_prefix() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let table = ArmsStockTable::new([11, 7, 23, 0, 0, 0, 0, 0], 3);
        let mut stock = make_stock();
        let mut gold = 1000u16;
        let ctx = ShopTransactionContext::default();

        step_arms_shop(
            &mut state,
            ArmsShopInput::Key(b'B'),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );

        assert_eq!(
            step_arms_shop(
                &mut state,
                ArmsShopInput::StockLetter {
                    letter: b'd',
                    table,
                },
                ctx,
                &mut gold,
                &mut stock,
                &prices,
            ),
            ArmsShopOutcome::InvalidInput
        );
        assert_eq!(state, ArmsShopState::BuyPickItem);
        assert_eq!(gold, 1000);
        assert!(stock.iter().all(|count| *count == 0));
    }

    #[test]
    fn arms_shop_sell_path_credits_gold_and_decrements_stock() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let mut stock = make_stock();
        stock[5] = 2;
        let mut gold = 0u16;
        let ctx = ShopTransactionContext {
            party_gold: 0,
            speaker_intelligence: 10,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };

        step_arms_shop(
            &mut state,
            ArmsShopInput::Key(b'S'),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        step_arms_shop(
            &mut state,
            ArmsShopInput::Item(5),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        let outcome = step_arms_shop(
            &mut state,
            ArmsShopInput::Confirm(true),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        assert!(matches!(outcome, ArmsShopOutcome::Sold { item: 5, .. }));
        assert_eq!(stock[5], 1);
        assert!(gold > 0);
    }

    #[test]
    fn arms_shop_buy_refused_when_short_funds() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let mut stock = make_stock();
        let mut gold = 1u16;
        let ctx = ShopTransactionContext {
            party_gold: 1,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };
        step_arms_shop(
            &mut state,
            ArmsShopInput::Key(b'B'),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        step_arms_shop(
            &mut state,
            ArmsShopInput::Item(5),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        let outcome = step_arms_shop(
            &mut state,
            ArmsShopInput::Confirm(true),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        assert!(matches!(
            outcome,
            ArmsShopOutcome::BuyRefusedShortFunds { .. }
        ));
        assert_eq!(gold, 1);
        assert_eq!(stock[5], 0);
    }

    #[test]
    fn arms_shop_sell_refused_when_no_stock() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let mut stock = make_stock();
        let mut gold = 0u16;
        let ctx = ShopTransactionContext::default();
        step_arms_shop(
            &mut state,
            ArmsShopInput::Key(b'S'),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        let outcome = step_arms_shop(
            &mut state,
            ArmsShopInput::Item(5),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        assert!(matches!(
            outcome,
            ArmsShopOutcome::SellRefusedNoStock { item: 5 }
        ));
    }

    #[test]
    fn arms_shop_decline_returns_to_greeting() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let mut stock = make_stock();
        let mut gold = 1000u16;
        let ctx = ShopTransactionContext::default();
        step_arms_shop(
            &mut state,
            ArmsShopInput::Key(b'B'),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        step_arms_shop(
            &mut state,
            ArmsShopInput::Item(5),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        let outcome = step_arms_shop(
            &mut state,
            ArmsShopInput::Confirm(false),
            ctx,
            &mut gold,
            &mut stock,
            &prices,
        );
        assert_eq!(outcome, ArmsShopOutcome::Declined);
        assert_eq!(gold, 1000);
        assert_eq!(stock[5], 0);
        assert_eq!(state, ArmsShopState::Greeting);
    }

    #[test]
    fn arms_shop_exit_on_unknown_key() {
        let mut state = ArmsShopState::Greeting;
        let prices = make_price_table();
        let mut stock = make_stock();
        let mut gold = 100u16;
        let outcome = step_arms_shop(
            &mut state,
            ArmsShopInput::Key(b' '),
            ShopTransactionContext::default(),
            &mut gold,
            &mut stock,
            &prices,
        );
        assert_eq!(outcome, ArmsShopOutcome::Exited);
    }

    #[test]
    fn healer_cure_requires_poison_status() {
        let mut state = HealerShopState::Greeting;
        let mut gold = 1000u16;
        let mut members = vec![HealerPartyMemberView {
            status: b'G',
            hp: 50,
            max_hp: 50,
        }];
        step_healer_shop(
            &mut state,
            HealerShopInput::Key(b'Y'),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Service(HealerService::Cure),
            &mut gold,
            &mut members,
        );
        let outcome = step_healer_shop(
            &mut state,
            HealerShopInput::Slot(0),
            &mut gold,
            &mut members,
        );
        assert!(matches!(
            outcome,
            HealerOutcome::RefusedNotEligible {
                service: HealerService::Cure,
                slot: 0
            }
        ));
    }

    #[test]
    fn healer_heal_restores_hp_and_charges_gold() {
        let mut state = HealerShopState::Greeting;
        let mut gold = 1000u16;
        let mut members = vec![HealerPartyMemberView {
            status: b'G',
            hp: 20,
            max_hp: 50,
        }];
        step_healer_shop(
            &mut state,
            HealerShopInput::Key(b'Y'),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Service(HealerService::Heal),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Slot(0),
            &mut gold,
            &mut members,
        );
        let outcome = step_healer_shop(
            &mut state,
            HealerShopInput::Confirm(true),
            &mut gold,
            &mut members,
        );
        assert!(matches!(
            outcome,
            HealerOutcome::Served {
                service: HealerService::Heal,
                slot: 0,
                ..
            }
        ));
        assert_eq!(members[0].hp, 50);
        assert_eq!(gold, 1000 - HEALER_COST_HEAL);
    }

    #[test]
    fn healer_heal_accepts_poisoned_member_without_curing_status() {
        let mut state = HealerShopState::Greeting;
        let mut gold = 1000u16;
        let mut members = vec![HealerPartyMemberView {
            status: b'P',
            hp: 20,
            max_hp: 50,
        }];
        step_healer_shop(
            &mut state,
            HealerShopInput::Key(b'Y'),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Service(HealerService::Heal),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Slot(0),
            &mut gold,
            &mut members,
        );

        let outcome = step_healer_shop(
            &mut state,
            HealerShopInput::Confirm(true),
            &mut gold,
            &mut members,
        );

        assert!(matches!(
            outcome,
            HealerOutcome::Served {
                service: HealerService::Heal,
                slot: 0,
                ..
            }
        ));
        assert_eq!(members[0].status, b'P');
        assert_eq!(members[0].hp, 50);
        assert_eq!(gold, 1000 - HEALER_COST_HEAL);
    }

    #[test]
    fn healer_resurrect_returns_dead_party_member_to_life() {
        let mut state = HealerShopState::Greeting;
        let mut gold = 1000u16;
        let mut members = vec![HealerPartyMemberView {
            status: b'D',
            hp: 0,
            max_hp: 50,
        }];
        step_healer_shop(
            &mut state,
            HealerShopInput::Key(b'Y'),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Service(HealerService::Resurrect),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Slot(0),
            &mut gold,
            &mut members,
        );
        let outcome = step_healer_shop(
            &mut state,
            HealerShopInput::Confirm(true),
            &mut gold,
            &mut members,
        );
        assert!(matches!(outcome, HealerOutcome::Served { .. }));
        assert_eq!(members[0].status, b'G');
        assert!(members[0].hp > 0);
    }

    #[test]
    fn healer_short_funds_refuses_without_consuming_gold() {
        let mut state = HealerShopState::Greeting;
        let mut gold = 10u16;
        let mut members = vec![HealerPartyMemberView {
            status: b'D',
            hp: 0,
            max_hp: 50,
        }];
        step_healer_shop(
            &mut state,
            HealerShopInput::Key(b'Y'),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Service(HealerService::Resurrect),
            &mut gold,
            &mut members,
        );
        step_healer_shop(
            &mut state,
            HealerShopInput::Slot(0),
            &mut gold,
            &mut members,
        );
        let outcome = step_healer_shop(
            &mut state,
            HealerShopInput::Confirm(true),
            &mut gold,
            &mut members,
        );
        assert!(matches!(outcome, HealerOutcome::RefusedShortFunds { .. }));
        assert_eq!(gold, 10);
        assert_eq!(members[0].status, b'D');
    }

    #[test]
    fn innkeeper_rest_path_quotes_public_room_rate() {
        let mut state = InnkeeperState::for_inn(Inn::TheWayfarerInn);
        let ctx = ShopTransactionContext {
            party_gold: 100,
            speaker_intelligence: 30,
            world_hour: 12,
            party_size: 2,
            living_party_members: 2,
        };

        let outcome = step_innkeeper(&mut state, InnkeeperInput::Key(b'R'), ctx);
        assert_eq!(
            outcome,
            InnkeeperOutcome::QuotedRest {
                inn: Inn::TheWayfarerInn,
                base_room_rate: 2,
                total_price: 4,
            }
        );

        let outcome = step_innkeeper(&mut state, InnkeeperInput::Confirm(true), ctx);
        assert!(matches!(
            outcome,
            InnkeeperOutcome::RestConfirmed {
                inn: Inn::TheWayfarerInn,
                total_price: 4,
                ..
            }
        ));
    }

    #[test]
    fn innkeeper_leave_path_quotes_deposit_and_target() {
        let mut state = InnkeeperState::for_inn(Inn::HotelBrittany);
        let ctx = ShopTransactionContext {
            party_gold: 100,
            speaker_intelligence: 30,
            world_hour: 12,
            party_size: 2,
            living_party_members: 2,
        };

        let outcome = step_innkeeper(&mut state, InnkeeperInput::Key(b'L'), ctx);
        assert_eq!(
            outcome,
            InnkeeperOutcome::PickLeaveCompanion { deposit: 33 }
        );

        let outcome = step_innkeeper(&mut state, InnkeeperInput::Slot(1), ctx);
        assert_eq!(
            outcome,
            InnkeeperOutcome::QuotedLeaveCompanion {
                party_index: 1,
                deposit: 33,
            }
        );
    }

    #[test]
    fn innkeeper_quotes_intelligence_adjusted_price_after_raw_bill() {
        let high_int = ShopTransactionContext {
            party_gold: 100,
            speaker_intelligence: 75,
            world_hour: 12,
            party_size: 4,
            living_party_members: 4,
        };
        let mut rest = InnkeeperState::for_inn(Inn::HotelBrittany);
        assert_eq!(
            step_innkeeper(&mut rest, InnkeeperInput::Key(b'R'), high_int),
            InnkeeperOutcome::QuotedRest {
                inn: Inn::HotelBrittany,
                base_room_rate: 3,
                total_price: 0,
            }
        );

        let mut leave = InnkeeperState::for_inn(Inn::HotelBrittany);
        assert_eq!(
            step_innkeeper(&mut leave, InnkeeperInput::Key(b'L'), high_int),
            InnkeeperOutcome::PickLeaveCompanion { deposit: 0 }
        );

        let mut pickup = InnkeeperState::PickUpCompanion {
            inn: Inn::HotelBrittany,
            guest_indices: [0; INN_REGISTRY_CAP],
            guest_count: 1,
            base_lodging_charge: 3,
        };
        assert_eq!(
            step_innkeeper(&mut pickup, InnkeeperInput::GuestChoice(0), high_int),
            InnkeeperOutcome::QuotedPickUpCompanion {
                registry_index: 0,
                bill: 0,
            }
        );
    }

    #[test]
    fn innkeeper_pickup_choice_can_carry_guest_stay_counter() {
        let ctx = ShopTransactionContext {
            party_gold: 100,
            speaker_intelligence: 30,
            world_hour: 12,
            party_size: 4,
            living_party_members: 4,
        };
        let mut state = InnkeeperState::PickUpCompanion {
            inn: Inn::HotelBrittany,
            guest_indices: [2; INN_REGISTRY_CAP],
            guest_count: 1,
            base_lodging_charge: 3,
        };

        assert_eq!(
            step_innkeeper(
                &mut state,
                InnkeeperInput::GuestChoiceWithStay {
                    choice: 0,
                    stay_counter: 5,
                },
                ctx,
            ),
            InnkeeperOutcome::QuotedPickUpCompanion {
                registry_index: 2,
                bill: 165,
            }
        );
    }

    #[test]
    fn tavern_menu_uses_published_round_letter_and_p_for_provisions() {
        let mut state = TavernState::for_tavern(Tavern::TheWayfarerTavern);
        let mut gold = 100u16;
        let mut food = 30u16;
        let ctx = ShopTransactionContext {
            party_gold: gold,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };

        assert_eq!(
            step_tavern(
                &mut state,
                TavernInput::Key(b'Y'),
                ctx,
                &mut gold,
                &mut food,
            ),
            TavernOutcome::EnteredMenu {
                tavern: Tavern::TheWayfarerTavern,
                round_letter: 'M',
                secondary_letter: 'A',
                provisions_letter: Some('R'),
                lore_letter: 'C',
            }
        );
        assert_eq!(
            step_tavern(
                &mut state,
                TavernInput::Key(b'Z'),
                ctx,
                &mut gold,
                &mut food,
            ),
            TavernOutcome::InvalidInput
        );
        assert_eq!(
            step_tavern(
                &mut state,
                TavernInput::Key(b'R'),
                ctx,
                &mut gold,
                &mut food,
            ),
            TavernOutcome::PickProvisionQuantity {
                tavern: Tavern::TheWayfarerTavern,
                unit_price: 15,
            }
        );
    }

    #[test]
    fn tavern_menu_lore_letters_enter_sage_after_documented_tavern_branches() {
        let ctx = ShopTransactionContext {
            party_gold: 100,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };
        let mut gold = 100u16;
        let mut food = 30u16;
        let mut state = TavernState::for_tavern(Tavern::TheWayfarerTavern);

        step_tavern(
            &mut state,
            TavernInput::Key(b'Y'),
            ctx,
            &mut gold,
            &mut food,
        );

        assert_eq!(
            step_tavern(
                &mut state,
                TavernInput::Key(b'A'),
                ctx,
                &mut gold,
                &mut food,
            ),
            TavernOutcome::SecondaryTavernSelected {
                tavern: Tavern::TheWayfarerTavern,
                letter: 'A',
            }
        );
        assert_eq!(
            step_tavern(
                &mut state,
                TavernInput::Key(b'C'),
                ctx,
                &mut gold,
                &mut food,
            ),
            TavernOutcome::EnteredSagePrompt
        );
        assert_eq!(state, TavernState::Exited);

        let mut blue_boar = TavernState::for_tavern(Tavern::TheBlueBoarTavern);
        step_tavern(
            &mut blue_boar,
            TavernInput::Key(b'Y'),
            ctx,
            &mut gold,
            &mut food,
        );
        assert_eq!(
            step_tavern(
                &mut blue_boar,
                TavernInput::Key(b'C'),
                ctx,
                &mut gold,
                &mut food,
            ),
            TavernOutcome::RoundDrinkServed {
                tavern: Tavern::TheBlueBoarTavern,
                cost: 5,
            }
        );
    }

    #[test]
    fn innkeeper_space_exits_from_greeting() {
        let mut state = InnkeeperState::default();
        let ctx = ShopTransactionContext {
            party_gold: 100,
            speaker_intelligence: 30,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };

        let outcome = step_innkeeper(&mut state, InnkeeperInput::Key(b' '), ctx);
        assert_eq!(outcome, InnkeeperOutcome::Exited);
        assert_eq!(state, InnkeeperState::Exited);
    }

    #[test]
    fn reagent_shop_compact_letter_path_debits_gold_and_increments_stock() {
        let mut state = ReagentShopState::for_herbalist(Herbalist::Mysticism);
        let mut gold = 1000u16;
        let mut stock = [0u8; 8];
        let quote = step_reagent_shop(
            &mut state,
            ReagentShopInput::Key(b'A'),
            &mut gold,
            &mut stock,
        );
        assert_eq!(
            quote,
            ReagentShopOutcome::QuotedUnit {
                herbalist: Herbalist::Mysticism,
                reagent: Reagent::SpiderSilk,
                unit_price: 6
            }
        );
        let outcome = step_reagent_shop(
            &mut state,
            ReagentShopInput::Quantity(4),
            &mut gold,
            &mut stock,
        );
        assert!(matches!(
            outcome,
            ReagentShopOutcome::Bought {
                herbalist: Herbalist::Mysticism,
                reagent: Reagent::SpiderSilk,
                quantity: 4,
                paid: 24
            }
        ));
        assert_eq!(stock[Reagent::SpiderSilk.inventory_index()], 4);
        assert_eq!(gold, 1000 - 24);
    }

    #[test]
    fn reagent_shop_zero_quantity_treated_as_decline() {
        let mut state = ReagentShopState::for_herbalist(Herbalist::TheHerbalist);
        let mut gold = 1000u16;
        let mut stock = [0u8; 8];
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Key(b'A'),
            &mut gold,
            &mut stock,
        );
        let outcome = step_reagent_shop(
            &mut state,
            ReagentShopInput::Quantity(0),
            &mut gold,
            &mut stock,
        );
        assert_eq!(outcome, ReagentShopOutcome::Declined);
        assert_eq!(gold, 1000);
    }

    #[test]
    fn reagent_shop_refuses_stock_cap_overflow_without_partial_mutation() {
        let mut state = ReagentShopState::for_herbalist(Herbalist::TheSharperMage);
        let mut gold = 60_000u16;
        let mut stock = [0u8; 8];
        stock[Reagent::BloodMoss.inventory_index()] = 95;
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Key(b'A'),
            &mut gold,
            &mut stock,
        );
        let outcome = step_reagent_shop(
            &mut state,
            ReagentShopInput::Quantity(20),
            &mut gold,
            &mut stock,
        );
        assert_eq!(
            outcome,
            ReagentShopOutcome::RefusedStockCap {
                current: 95,
                requested: 20,
                cap: REAGENT_STOCK_CAP_PER_KIND
            }
        );
        assert_eq!(stock[Reagent::BloodMoss.inventory_index()], 95);
        assert_eq!(gold, 60_000);
    }

    #[test]
    fn tavern_round_drink_charges_per_living_member() {
        let mut state = TavernState::for_tavern(Tavern::TheSwordAndKeg);
        let mut gold = 100u16;
        let mut food = 30u16;
        let ctx = ShopTransactionContext {
            party_gold: gold,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 3,
            living_party_members: 3,
        };

        let entered = step_tavern(
            &mut state,
            TavernInput::Key(b'Y'),
            ctx,
            &mut gold,
            &mut food,
        );
        assert_eq!(
            entered,
            TavernOutcome::EnteredMenu {
                tavern: Tavern::TheSwordAndKeg,
                round_letter: 'M',
                secondary_letter: 'A',
                provisions_letter: Some('R'),
                lore_letter: 'C',
            }
        );
        let outcome = step_tavern(
            &mut state,
            TavernInput::Key(b'M'),
            ctx,
            &mut gold,
            &mut food,
        );
        assert_eq!(
            outcome,
            TavernOutcome::RoundDrinkServed {
                tavern: Tavern::TheSwordAndKeg,
                cost: 15,
            }
        );
        assert_eq!(food, 30);
        assert_eq!(gold, 85);
    }

    #[test]
    fn tavern_blue_boar_w_branch_sells_fixed_drink_choice() {
        let mut state = TavernState::for_tavern(Tavern::TheBlueBoarTavern);
        let mut gold = 200u16;
        let mut food = 30u16;
        let ctx = ShopTransactionContext {
            party_gold: gold,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };

        step_tavern(
            &mut state,
            TavernInput::Key(b'Y'),
            ctx,
            &mut gold,
            &mut food,
        );
        let list = step_tavern(
            &mut state,
            TavernInput::Key(b'W'),
            ctx,
            &mut gold,
            &mut food,
        );
        assert_eq!(list, TavernOutcome::PickBlueBoarDrink);
        let outcome = step_tavern(
            &mut state,
            TavernInput::Key(b'F'),
            ctx,
            &mut gold,
            &mut food,
        );
        assert_eq!(
            outcome,
            TavernOutcome::BlueBoarDrinkServed {
                choice: BlueBoarDrinkChoice::F,
                cost: 98,
            }
        );
        assert_eq!(gold, 102);
    }

    #[test]
    fn tavern_drink_short_funds_refuses() {
        let mut state = TavernState::for_tavern(Tavern::TheSwordAndKeg);
        let mut gold = 1u16;
        let mut food = 0u16;
        let ctx = ShopTransactionContext {
            party_gold: gold,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };
        step_tavern(
            &mut state,
            TavernInput::Key(b'Y'),
            ctx,
            &mut gold,
            &mut food,
        );
        let outcome = step_tavern(
            &mut state,
            TavernInput::Key(b'M'),
            ctx,
            &mut gold,
            &mut food,
        );
        assert!(matches!(outcome, TavernOutcome::RefusedShortFunds { .. }));
        assert_eq!(gold, 1);
    }

    #[test]
    fn tavern_provisions_purchase_can_partially_fill_requested_quantity() {
        let mut state = TavernState::for_tavern(Tavern::TheWayfarerTavern);
        let mut gold = 100u16;
        let mut food = crate::SHOP_FOOD_STOCK_CAP - 2;
        let ctx = ShopTransactionContext {
            party_gold: gold,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };
        step_tavern(
            &mut state,
            TavernInput::Key(b'Y'),
            ctx,
            &mut gold,
            &mut food,
        );
        let prompt = step_tavern(
            &mut state,
            TavernInput::Key(b'R'),
            ctx,
            &mut gold,
            &mut food,
        );
        assert_eq!(
            prompt,
            TavernOutcome::PickProvisionQuantity {
                tavern: Tavern::TheWayfarerTavern,
                unit_price: 15,
            }
        );
        let outcome = step_tavern(
            &mut state,
            TavernInput::Quantity(5),
            ctx,
            &mut gold,
            &mut food,
        );
        assert_eq!(
            outcome,
            TavernOutcome::ProvisionsPurchased {
                tavern: Tavern::TheWayfarerTavern,
                requested_quantity: 5,
                purchased_quantity: 2,
                paid: 30,
                food_added: 2,
            }
        );
        assert_eq!(gold, 70);
        assert_eq!(food, crate::SHOP_FOOD_STOCK_CAP);
        assert_eq!(
            state,
            TavernState::Menu {
                tavern: Tavern::TheWayfarerTavern,
                continuation_ready: true,
            }
        );
    }

    #[test]
    fn tavern_zero_provision_quantity_declines_without_state_change() {
        let mut state = TavernState::for_tavern(Tavern::TheHonestMeal);
        let mut gold = 100u16;
        let mut food = 30u16;
        let ctx = ShopTransactionContext {
            party_gold: gold,
            speaker_intelligence: 0,
            world_hour: 12,
            party_size: 1,
            living_party_members: 1,
        };

        step_tavern(
            &mut state,
            TavernInput::Key(b'Y'),
            ctx,
            &mut gold,
            &mut food,
        );
        step_tavern(
            &mut state,
            TavernInput::Key(b'R'),
            ctx,
            &mut gold,
            &mut food,
        );
        let outcome = step_tavern(
            &mut state,
            TavernInput::Quantity(0),
            ctx,
            &mut gold,
            &mut food,
        );

        assert_eq!(outcome, TavernOutcome::Declined);
        assert_eq!(gold, 100);
        assert_eq!(food, 30);
        assert_eq!(
            state,
            TavernState::Menu {
                tavern: Tavern::TheHonestMeal,
                continuation_ready: false,
            }
        );
    }

    #[test]
    fn sage_rumour_keyword_match_quotes_then_debits_on_confirmation() {
        let mut state = SageState::default();
        let mut gold = 60u16;

        let outcome = step_sage(&mut state, SageInput::Keyword("HONE clue"), &mut gold);

        assert_eq!(
            outcome,
            SageOutcome::QuotedFee {
                quote: SageRumourQuote {
                    entry: SAGE_RUMOUR_TABLE[0],
                    input_len: 9,
                },
            }
        );
        assert_eq!(gold, 60);
        assert_eq!(
            state,
            SageState::Confirm {
                table: &SAGE_RUMOUR_TABLE,
                quote: SageRumourQuote {
                    entry: SAGE_RUMOUR_TABLE[0],
                    input_len: 9,
                }
            }
        );

        let outcome = step_sage(
            &mut state,
            SageInput::Confirm {
                accepted: true,
                record_id: SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
            },
            &mut gold,
        );

        assert_eq!(gold, 10);
        assert_eq!(
            outcome,
            SageOutcome::RumourFound {
                outcome: SageRumourOutcome {
                    quote: SageRumourQuote {
                        entry: SAGE_RUMOUR_TABLE[0],
                        input_len: 9,
                    },
                    record_id: SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
                    rendered: "Seek ye Malik in Moonglow!".to_string(),
                },
                rendered: "Seek ye Malik in Moonglow!".to_string(),
            }
        );
        assert_eq!(state, SageState::Exited);
    }

    #[test]
    fn sage_rumour_refusals_and_empty_exit_preserve_gold() {
        let mut state = SageState::default();
        let mut gold = 10u16;

        assert_eq!(
            step_sage(&mut state, SageInput::Keyword("honesty"), &mut gold),
            SageOutcome::NoTopicMatch
        );
        assert_eq!(gold, 10);
        assert_eq!(
            state,
            SageState::Prompt {
                table: &SAGE_RUMOUR_TABLE
            }
        );

        assert_eq!(
            step_sage(&mut state, SageInput::Keyword("hone"), &mut gold),
            SageOutcome::QuotedFee {
                quote: SageRumourQuote {
                    entry: SAGE_RUMOUR_TABLE[0],
                    input_len: 4,
                }
            }
        );
        assert_eq!(
            step_sage(
                &mut state,
                SageInput::Confirm {
                    accepted: true,
                    record_id: SAGE_RUMOUR_SUCCESS_RECORD_FIRST,
                },
                &mut gold,
            ),
            SageOutcome::RefusedShortFunds {
                required: 50,
                available: 10,
            }
        );
        assert_eq!(gold, 10);
        assert_eq!(state, SageState::Exited);

        assert_eq!(
            step_sage(&mut state, SageInput::Keyword(" "), &mut gold),
            SageOutcome::Exited
        );
        assert_eq!(state, SageState::Exited);
    }

    #[test]
    fn horse_trader_purchase_path_requires_local_marker_and_charges_adjusted_price() {
        let mut state = HorseTraderState::default();
        let mut gold = 500u16;
        assert_eq!(
            step_horse_trader(
                &mut state,
                HorseTraderInput::Key {
                    key: b'Y',
                    speaker_intelligence: 30,
                },
                &mut gold,
            ),
            HorseTraderOutcome::QuotedPrice { price: 110 }
        );
        let outcome = step_horse_trader(
            &mut state,
            HorseTraderInput::Confirm {
                accepted: true,
                can_place_horse: true,
            },
            &mut gold,
        );
        assert_eq!(outcome, HorseTraderOutcome::Purchased { price: 110 });
        assert_eq!(gold, 390);
    }

    #[test]
    fn horse_trader_refuses_when_no_adjacent_marker_is_available() {
        let mut state = HorseTraderState::for_stable(Stable::WishingWellHorses);
        let mut gold = 500u16;
        assert_eq!(
            step_horse_trader(
                &mut state,
                HorseTraderInput::Key {
                    key: b'B',
                    speaker_intelligence: 40,
                },
                &mut gold,
            ),
            HorseTraderOutcome::QuotedPrice { price: 128 }
        );
        assert_eq!(
            step_horse_trader(
                &mut state,
                HorseTraderInput::Confirm {
                    accepted: true,
                    can_place_horse: false,
                },
                &mut gold,
            ),
            HorseTraderOutcome::RefusedNoMarker { price: 128 }
        );
        assert_eq!(gold, 500);
        assert_eq!(
            state,
            HorseTraderState::Greeting {
                stable: Stable::WishingWellHorses
            }
        );
    }

    #[test]
    fn horse_trader_short_funds_refuses() {
        let mut state = HorseTraderState::default();
        let mut gold = 10u16;
        step_horse_trader(
            &mut state,
            HorseTraderInput::Key {
                key: b'Y',
                speaker_intelligence: 30,
            },
            &mut gold,
        );
        let outcome = step_horse_trader(
            &mut state,
            HorseTraderInput::Confirm {
                accepted: true,
                can_place_horse: true,
            },
            &mut gold,
        );
        assert!(matches!(
            outcome,
            HorseTraderOutcome::RefusedShortFunds { .. }
        ));
        assert_eq!(gold, 10);
    }

    #[test]
    fn ship_broker_f_key_quotes_then_queues_frigate_delivery() {
        let mut state = ShipBrokerState::for_shipwright(Shipwright::TheRustyBucket);
        let mut gold = 700u16;
        let mut pending = None;
        let quote = step_ship_broker(
            &mut state,
            ShipBrokerInput::Key(b'F'),
            &mut gold,
            &mut pending,
        );
        assert!(matches!(
            quote,
            ShipBrokerOutcome::QuotedPurchase {
                quote: ShipwrightPurchaseQuote {
                    shipwright: Shipwright::TheRustyBucket,
                    kind: crate::shops::ShipwrightPurchaseKind::Frigate,
                    price: 700,
                }
            }
        ));

        let outcome = step_ship_broker(
            &mut state,
            ShipBrokerInput::Confirm(true),
            &mut gold,
            &mut pending,
        );
        assert!(matches!(
            outcome,
            ShipBrokerOutcome::PurchaseApplied {
                outcome: ShipwrightPurchaseOutcome {
                    status: crate::shops::ShipwrightPurchaseStatus::QueuedFrigate,
                    ..
                }
            }
        ));
        assert_eq!(gold, 0);
        assert_eq!(
            pending,
            Some(PendingVehicleAcquisition::Frigate {
                x: 136,
                y: 158,
                skiffs: 2,
            })
        );
    }

    #[test]
    fn ship_broker_s_key_adds_skiff_to_pending_frigate() {
        let mut state = ShipBrokerState::for_shipwright(Shipwright::TheOakenOar);
        let mut gold = 200u16;
        let mut pending = Some(PendingVehicleAcquisition::Frigate {
            x: 12,
            y: 21,
            skiffs: 2,
        });
        step_ship_broker(
            &mut state,
            ShipBrokerInput::Key(b'S'),
            &mut gold,
            &mut pending,
        );
        let outcome = step_ship_broker(
            &mut state,
            ShipBrokerInput::Confirm(true),
            &mut gold,
            &mut pending,
        );
        assert!(matches!(
            outcome,
            ShipBrokerOutcome::PurchaseApplied {
                outcome: ShipwrightPurchaseOutcome {
                    status: crate::shops::ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate,
                    ..
                }
            }
        ));
        assert_eq!(gold, 75);
        assert_eq!(
            pending,
            Some(PendingVehicleAcquisition::Frigate {
                x: 12,
                y: 21,
                skiffs: 3,
            })
        );
    }

    #[test]
    fn guild_shop_letter_path_debits_gold_and_increments_count() {
        let mut state = GuildShopState::for_shop(GuildShop::TheDen);
        let mut gold = 1000u16;
        let mut gems = 0u8;
        let mut keys = 0u8;
        let mut torches = 0u8;
        let quote = step_guild_shop(
            &mut state,
            GuildShopInput::Key(b'A'),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
        );
        assert_eq!(
            quote,
            GuildShopOutcome::QuotedUnit {
                shop: GuildShop::TheDen,
                commodity: GuildCommodity::Keys,
                unit_price: 190
            }
        );
        let outcome = step_guild_shop(
            &mut state,
            GuildShopInput::Quantity(2),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
        );
        assert!(matches!(
            outcome,
            GuildShopOutcome::Bought {
                shop: GuildShop::TheDen,
                commodity: GuildCommodity::Keys,
                quantity: 2,
                paid: 380
            }
        ));
        assert_eq!(keys, 2);
        assert_eq!(gems, 0);
        assert_eq!(gold, 620);
    }

    #[test]
    fn guild_shop_non_menu_letter_exits_without_selling_sextant_placeholder() {
        let mut state = GuildShopState::for_shop(GuildShop::TheGuild);
        let mut gold = 5000u16;
        let mut gems = 0u8;
        let mut keys = 0u8;
        let mut torches = 0u8;
        let outcome = step_guild_shop(
            &mut state,
            GuildShopInput::Key(b'X'),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
        );
        // Non-menu guild letters exit instead of buying placeholder goods.
        assert_eq!(outcome, GuildShopOutcome::Exited);
        assert_eq!(state, GuildShopState::Exited);
        assert_eq!(gold, 5000);
        assert_eq!((gems, keys, torches), (0, 0, 0));
    }

    #[test]
    fn guild_shop_short_funds_refuses_quantity_purchase() {
        let mut state = GuildShopState::for_shop(GuildShop::TheGuild);
        let mut gold = 100u16;
        let mut gems = 0u8;
        let mut keys = 0u8;
        let mut torches = 0u8;
        step_guild_shop(
            &mut state,
            GuildShopInput::Key(b'B'),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
        );
        let outcome = step_guild_shop(
            &mut state,
            GuildShopInput::Quantity(1),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
        );
        assert_eq!(outcome, GuildShopOutcome::RefusedShortFunds { cost: 200 });
        assert_eq!(gold, 100);
        assert_eq!(gems, 0);
    }

    #[test]
    fn healer_service_eligibility_table_matches_status_and_hp_rules() {
        let good_full = HealerPartyMemberView {
            status: b'G',
            hp: 50,
            max_hp: 50,
        };
        let good_low = HealerPartyMemberView {
            status: b'G',
            hp: 10,
            max_hp: 50,
        };
        let poisoned = HealerPartyMemberView {
            status: b'P',
            hp: 30,
            max_hp: 50,
        };
        let dead = HealerPartyMemberView {
            status: b'D',
            hp: 0,
            max_hp: 50,
        };
        assert!(!healer_service_eligible(HealerService::Cure, good_full));
        assert!(healer_service_eligible(HealerService::Cure, poisoned));
        assert!(!healer_service_eligible(HealerService::Heal, good_full));
        assert!(healer_service_eligible(HealerService::Heal, good_low));
        assert!(healer_service_eligible(HealerService::Heal, poisoned));
        assert!(!healer_service_eligible(HealerService::Heal, dead));
        assert!(healer_service_eligible(HealerService::Resurrect, dead));
        assert!(!healer_service_eligible(
            HealerService::Resurrect,
            good_full
        ));
    }
}
