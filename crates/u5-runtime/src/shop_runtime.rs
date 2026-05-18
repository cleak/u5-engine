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
    ArmsShopAction, BlueBoarDrinkChoice, INN_REGISTRY_CAP, Inn, InnMainAction,
    ProvisionPurchaseError, Shipwright, ShipwrightMenuAction, ShipwrightPurchaseError,
    ShipwrightPurchaseOutcome, ShipwrightPurchaseQuote, Tavern, TavernDrinkError,
    TavernDrinkPrompt, apply_blue_boar_drink, apply_provision_purchase, apply_shipwright_purchase,
    apply_tavern_round_drink, arms_shop_action, arms_shop_buy_quote, arms_shop_sell_offer,
    inn_base_room_rate, inn_leave_companion_deposit, inn_main_action, inn_pickup_bill,
    quote_inn_rest, quote_shipwright_purchase, shipwright_menu_action, tavern_drink_prompt,
    tavern_provision_unit_price, tavern_round_drink_menu_letter,
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
    BuyConfirm { item: u8, quoted_price: u16 },
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
    QuotedBuyPrice { item: u8, price: u16 },
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
            let item_idx = item as usize;
            if item_idx >= EQUIPMENT_COUNT {
                return ArmsShopOutcome::InvalidInput;
            }
            let base = base_price_table[item_idx];
            if base == 0 {
                // Items the shop does not stock at all are treated as
                // an invalid pick rather than an offer of free goods.
                return ArmsShopOutcome::InvalidInput;
            }
            let price = arms_shop_buy_quote(base, ctx.speaker_intelligence);
            *state = ArmsShopState::BuyConfirm {
                item,
                quoted_price: price,
            };
            ArmsShopOutcome::QuotedBuyPrice { item, price }
        }
        (ArmsShopState::BuyConfirm { item, quoted_price }, ArmsShopInput::Confirm(true)) => {
            if *gold < quoted_price {
                *state = ArmsShopState::Greeting;
                return ArmsShopOutcome::BuyRefusedShortFunds { item, quoted_price };
            }
            let item_idx = item as usize;
            if stock[item_idx] >= EQUIPMENT_STOCK_CAP {
                *state = ArmsShopState::Greeting;
                return ArmsShopOutcome::BuyRefusedCapHit { item };
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

// ---------- Healer ----------

/// Healer / sanctum services per `shops.md §8.5`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerService {
    /// `C` — Cure poison. Flat cost.
    Cure,
    /// `H` — Heal HP. Flat cost; restores party member to max HP.
    Heal,
    /// `R` — Resurrect dead member. Highest cost.
    Resurrect,
}

/// Default healer service costs (gold) used when a per-shop override
/// is not supplied. Values are first-playable approximations of the
/// commonly-quoted vanilla v1 prices.
pub const HEALER_COST_CURE: u16 = 100;
pub const HEALER_COST_HEAL: u16 = 200;
pub const HEALER_COST_RESURRECT: u16 = 300;

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
        HealerService::Heal => member.status == b'G' && member.hp < member.max_hp,
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
            if member.status == b'G' {
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
        adjusted_room_rate: u16,
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
        adjusted_lodging_charge: u16,
    },
    ConfirmPickUpCompanion {
        inn: Inn,
        registry_index: usize,
        adjusted_lodging_charge: u16,
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
    Confirm(bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InnkeeperOutcome {
    QuotedRest {
        inn: Inn,
        adjusted_room_rate: u16,
        total_price: u16,
    },
    RestConfirmed {
        inn: Inn,
        adjusted_room_rate: u16,
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
                let adjusted_room_rate = inn_base_room_rate(inn);
                let total_price = quote_inn_rest(inn, ctx.party_size, adjusted_room_rate)
                    .map(|quote| quote.total_price)
                    .unwrap_or(0);
                *state = InnkeeperState::ConfirmRest {
                    inn,
                    adjusted_room_rate,
                    total_price,
                };
                InnkeeperOutcome::QuotedRest {
                    inn,
                    adjusted_room_rate,
                    total_price,
                }
            }
            InnMainAction::LeaveCompanion => {
                let deposit = inn_leave_companion_deposit(inn_base_room_rate(inn));
                *state = InnkeeperState::PickLeaveCompanion { inn, deposit };
                InnkeeperOutcome::PickLeaveCompanion { deposit }
            }
            InnMainAction::PickUpCompanion => {
                *state = InnkeeperState::PickUpCompanion {
                    inn,
                    guest_indices: [0; INN_REGISTRY_CAP],
                    guest_count: 0,
                    adjusted_lodging_charge: inn_leave_companion_deposit(inn_base_room_rate(inn)),
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
                adjusted_room_rate,
                total_price,
            },
            InnkeeperInput::Confirm(true),
        ) => {
            *state = InnkeeperState::Greeting { inn };
            InnkeeperOutcome::RestConfirmed {
                inn,
                adjusted_room_rate,
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
                adjusted_lodging_charge,
            },
            InnkeeperInput::GuestChoice(choice),
        ) if choice < guest_count as usize => {
            let registry_index = guest_indices[choice];
            let bill = inn_pickup_bill(adjusted_lodging_charge, 1);
            *state = InnkeeperState::ConfirmPickUpCompanion {
                inn,
                registry_index,
                adjusted_lodging_charge,
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
                adjusted_lodging_charge: _,
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

/// Reagent shop transaction. The player picks one reagent and a
/// quantity; shop charges `unit_price * quantity` and adds the stock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ReagentShopState {
    #[default]
    Greeting,
    PickReagent,
    PickQuantity {
        reagent: u8,
        unit_price: u16,
    },
    Confirm {
        reagent: u8,
        quantity: u8,
        total: u16,
    },
    Exited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReagentShopInput {
    Key(u8),
    /// 0-based reagent index (0..REAGENT_COUNT).
    Reagent(u8),
    Quantity(u8),
    Confirm(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReagentShopOutcome {
    EnteredMenu,
    QuotedUnit {
        reagent: u8,
        unit_price: u16,
    },
    QuotedTotal {
        reagent: u8,
        quantity: u8,
        total: u16,
    },
    Bought {
        reagent: u8,
        quantity: u8,
        paid: u16,
    },
    RefusedShortFunds {
        total: u16,
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
    unit_price_table: &[u16],
) -> ReagentShopOutcome {
    match (*state, input) {
        (ReagentShopState::Greeting, ReagentShopInput::Key(b)) => match b {
            b'Y' | b'y' | b'B' | b'b' => {
                *state = ReagentShopState::PickReagent;
                ReagentShopOutcome::EnteredMenu
            }
            _ => {
                *state = ReagentShopState::Exited;
                ReagentShopOutcome::Exited
            }
        },
        (ReagentShopState::PickReagent, ReagentShopInput::Reagent(reagent)) => {
            let idx = reagent as usize;
            if idx >= reagent_stock.len() || idx >= unit_price_table.len() {
                return ReagentShopOutcome::InvalidInput;
            }
            let unit_price = unit_price_table[idx];
            if unit_price == 0 {
                return ReagentShopOutcome::InvalidInput;
            }
            *state = ReagentShopState::PickQuantity {
                reagent,
                unit_price,
            };
            ReagentShopOutcome::QuotedUnit {
                reagent,
                unit_price,
            }
        }
        (
            ReagentShopState::PickQuantity {
                reagent,
                unit_price,
            },
            ReagentShopInput::Quantity(quantity),
        ) => {
            if quantity == 0 {
                *state = ReagentShopState::Greeting;
                return ReagentShopOutcome::Declined;
            }
            let total = unit_price.saturating_mul(quantity as u16);
            *state = ReagentShopState::Confirm {
                reagent,
                quantity,
                total,
            };
            ReagentShopOutcome::QuotedTotal {
                reagent,
                quantity,
                total,
            }
        }
        (
            ReagentShopState::Confirm {
                reagent,
                quantity,
                total,
            },
            ReagentShopInput::Confirm(true),
        ) => {
            let idx = reagent as usize;
            if *gold < total {
                *state = ReagentShopState::Greeting;
                return ReagentShopOutcome::RefusedShortFunds { total };
            }
            *gold -= total;
            let new_stock = reagent_stock[idx]
                .saturating_add(quantity)
                .min(REAGENT_STOCK_CAP_PER_KIND);
            reagent_stock[idx] = new_stock;
            *state = ReagentShopState::Greeting;
            ReagentShopOutcome::Bought {
                reagent,
                quantity,
                paid: total,
            }
        }
        (ReagentShopState::Confirm { .. }, ReagentShopInput::Confirm(false)) => {
            *state = ReagentShopState::Greeting;
            ReagentShopOutcome::Declined
        }
        (ReagentShopState::Exited, _) => ReagentShopOutcome::Exited,
        _ => ReagentShopOutcome::InvalidInput,
    }
}

// ---------- Tavern ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TavernState {
    Greeting { tavern: Tavern },
    Menu { tavern: Tavern },
    PickProvisionQuantity { tavern: Tavern, unit_price: u16 },
    BlueBoarDrinkList { tavern: Tavern },
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
    },
    RoundDrinkServed {
        tavern: Tavern,
        cost: u16,
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
    RefusedShortFunds {
        cost: u16,
    },
    RefusedNoLivingParty,
    RefusedNoNeed,
    Exited,
    InvalidInput,
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
                *state = TavernState::Menu { tavern };
                TavernOutcome::EnteredMenu {
                    tavern,
                    round_letter: tavern_round_drink_menu_letter(tavern),
                }
            }
            TavernDrinkPrompt::Leave => {
                *state = TavernState::Exited;
                TavernOutcome::Exited
            }
            TavernDrinkPrompt::Discard => TavernOutcome::InvalidInput,
        },
        (TavernState::Menu { tavern }, TavernInput::Key(b)) => {
            let upper = b.to_ascii_uppercase();
            if upper == b' ' || upper == 0x1B || upper == b'N' {
                *state = TavernState::Exited;
                return TavernOutcome::Exited;
            }
            if upper == tavern_round_drink_menu_letter(tavern) as u8 {
                let outcome = apply_tavern_round_drink(gold, tavern, ctx.living_party_members);
                *state = TavernState::Menu { tavern };
                return match outcome {
                    Ok(outcome) => TavernOutcome::RoundDrinkServed {
                        tavern,
                        cost: outcome.total_price,
                    },
                    Err(TavernDrinkError::NoLivingParty) => TavernOutcome::RefusedNoLivingParty,
                    Err(TavernDrinkError::InsufficientGold { required, .. }) => {
                        TavernOutcome::RefusedShortFunds { cost: required }
                    }
                };
            }
            if matches!(tavern, Tavern::TheBlueBoarTavern) && upper == b'W' {
                *state = TavernState::BlueBoarDrinkList { tavern };
                return TavernOutcome::PickBlueBoarDrink;
            }
            if upper == b'P' {
                let unit_price = tavern_provision_unit_price(tavern);
                *state = TavernState::PickProvisionQuantity { tavern, unit_price };
                return TavernOutcome::PickProvisionQuantity { tavern, unit_price };
            }
            TavernOutcome::InvalidInput
        }
        (TavernState::PickProvisionQuantity { tavern, .. }, TavernInput::Quantity(quantity)) => {
            let outcome = apply_provision_purchase(gold, food, tavern, quantity);
            *state = TavernState::Menu { tavern };
            match outcome {
                Ok(outcome) => TavernOutcome::ProvisionsPurchased {
                    tavern,
                    requested_quantity: outcome.requested_quantity,
                    purchased_quantity: outcome.purchased_quantity,
                    paid: outcome.total_price,
                    food_added: outcome.food_after.saturating_sub(outcome.food_before),
                },
                Err(ProvisionPurchaseError::ZeroQuantity) => TavernOutcome::InvalidInput,
                Err(ProvisionPurchaseError::NoNeed) => TavernOutcome::RefusedNoNeed,
                Err(ProvisionPurchaseError::InsufficientGold {
                    required_per_unit, ..
                }) => TavernOutcome::RefusedShortFunds {
                    cost: required_per_unit,
                },
            }
        }
        (TavernState::BlueBoarDrinkList { tavern }, TavernInput::Key(b)) => {
            let Some(choice) = blue_boar_choice_for_key(b) else {
                return TavernOutcome::InvalidInput;
            };
            let outcome = apply_blue_boar_drink(gold, choice);
            *state = TavernState::Menu { tavern };
            match outcome {
                Ok(outcome) => TavernOutcome::BlueBoarDrinkServed {
                    choice,
                    cost: outcome.total_price,
                },
                Err(TavernDrinkError::NoLivingParty) => TavernOutcome::RefusedNoLivingParty,
                Err(TavernDrinkError::InsufficientGold { required, .. }) => {
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

// ---------- Horse trader ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HorseTraderState {
    #[default]
    Greeting,
    ConfirmPurchase {
        price: u16,
    },
    Exited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorseTraderInput {
    Key(u8),
    Confirm(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorseTraderOutcome {
    QuotedPrice { price: u16 },
    Purchased { price: u16 },
    RefusedShortFunds { price: u16 },
    Declined,
    Exited,
    InvalidInput,
}

pub const HORSE_TRADER_DEFAULT_PRICE: u16 = 250;

pub fn step_horse_trader(
    state: &mut HorseTraderState,
    input: HorseTraderInput,
    gold: &mut u16,
    horse_delivery_pending: &mut bool,
) -> HorseTraderOutcome {
    match (*state, input) {
        (HorseTraderState::Greeting, HorseTraderInput::Key(b)) => match b {
            b'Y' | b'y' | b'B' | b'b' => {
                *state = HorseTraderState::ConfirmPurchase {
                    price: HORSE_TRADER_DEFAULT_PRICE,
                };
                HorseTraderOutcome::QuotedPrice {
                    price: HORSE_TRADER_DEFAULT_PRICE,
                }
            }
            _ => {
                *state = HorseTraderState::Exited;
                HorseTraderOutcome::Exited
            }
        },
        (HorseTraderState::ConfirmPurchase { price }, HorseTraderInput::Confirm(true)) => {
            if *gold < price {
                *state = HorseTraderState::Greeting;
                return HorseTraderOutcome::RefusedShortFunds { price };
            }
            *gold -= price;
            *horse_delivery_pending = true;
            *state = HorseTraderState::Exited;
            HorseTraderOutcome::Purchased { price }
        }
        (HorseTraderState::ConfirmPurchase { .. }, HorseTraderInput::Confirm(false)) => {
            *state = HorseTraderState::Greeting;
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
    delivery_x: usize,
    delivery_y: usize,
) -> ShipBrokerOutcome {
    match (*state, input) {
        (ShipBrokerState::Greeting { shipwright }, ShipBrokerInput::Key(b)) => {
            match shipwright_menu_action(b) {
                ShipwrightMenuAction::Purchase(kind) => {
                    let quote = quote_shipwright_purchase(shipwright, kind);
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

/// Guild items: gems, keys, torches, sextant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildItem {
    Gems,
    Keys,
    Torches,
    Sextant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GuildShopState {
    #[default]
    Greeting,
    PickItem,
    PickQuantity {
        item: GuildItem,
        unit_price: u16,
    },
    ConfirmSextant {
        price: u16,
    },
    Exited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildShopInput {
    Key(u8),
    Item(GuildItem),
    Quantity(u8),
    Confirm(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildShopOutcome {
    EnteredMenu,
    QuotedUnit {
        item: GuildItem,
        unit_price: u16,
    },
    QuotedTotal {
        item: GuildItem,
        quantity: u8,
        total: u16,
    },
    Bought {
        item: GuildItem,
        quantity: u8,
        paid: u16,
    },
    SextantPurchased {
        price: u16,
    },
    RefusedShortFunds {
        cost: u16,
    },
    Declined,
    Exited,
    InvalidInput,
}

pub const GUILD_PRICE_GEMS_EACH: u16 = 100;
pub const GUILD_PRICE_KEYS_EACH: u16 = 30;
pub const GUILD_PRICE_TORCHES_EACH: u16 = 10;
pub const GUILD_PRICE_SEXTANT: u16 = 900;

pub const fn guild_item_unit_price(item: GuildItem) -> u16 {
    match item {
        GuildItem::Gems => GUILD_PRICE_GEMS_EACH,
        GuildItem::Keys => GUILD_PRICE_KEYS_EACH,
        GuildItem::Torches => GUILD_PRICE_TORCHES_EACH,
        GuildItem::Sextant => GUILD_PRICE_SEXTANT,
    }
}

pub fn step_guild_shop(
    state: &mut GuildShopState,
    input: GuildShopInput,
    gold: &mut u16,
    gems: &mut u8,
    keys: &mut u8,
    torches: &mut u8,
    sextant_owned: &mut bool,
) -> GuildShopOutcome {
    match (*state, input) {
        (GuildShopState::Greeting, GuildShopInput::Key(b)) => match b {
            b'Y' | b'y' | b'B' | b'b' => {
                *state = GuildShopState::PickItem;
                GuildShopOutcome::EnteredMenu
            }
            _ => {
                *state = GuildShopState::Exited;
                GuildShopOutcome::Exited
            }
        },
        (GuildShopState::PickItem, GuildShopInput::Item(GuildItem::Sextant)) => {
            if *sextant_owned {
                *state = GuildShopState::Greeting;
                return GuildShopOutcome::Declined;
            }
            let price = GUILD_PRICE_SEXTANT;
            *state = GuildShopState::ConfirmSextant { price };
            GuildShopOutcome::QuotedUnit {
                item: GuildItem::Sextant,
                unit_price: price,
            }
        }
        (GuildShopState::PickItem, GuildShopInput::Item(item)) => {
            let unit_price = guild_item_unit_price(item);
            *state = GuildShopState::PickQuantity { item, unit_price };
            GuildShopOutcome::QuotedUnit { item, unit_price }
        }
        (GuildShopState::PickQuantity { item, unit_price }, GuildShopInput::Quantity(quantity)) => {
            if quantity == 0 {
                *state = GuildShopState::Greeting;
                return GuildShopOutcome::Declined;
            }
            let total = unit_price.saturating_mul(quantity as u16);
            if *gold < total {
                *state = GuildShopState::Greeting;
                return GuildShopOutcome::RefusedShortFunds { cost: total };
            }
            *gold -= total;
            match item {
                GuildItem::Gems => *gems = gems.saturating_add(quantity),
                GuildItem::Keys => *keys = keys.saturating_add(quantity),
                GuildItem::Torches => *torches = torches.saturating_add(quantity),
                GuildItem::Sextant => unreachable!(),
            }
            *state = GuildShopState::Greeting;
            GuildShopOutcome::Bought {
                item,
                quantity,
                paid: total,
            }
        }
        (GuildShopState::ConfirmSextant { price }, GuildShopInput::Confirm(true)) => {
            if *gold < price {
                *state = GuildShopState::Greeting;
                return GuildShopOutcome::RefusedShortFunds { cost: price };
            }
            *gold -= price;
            *sextant_owned = true;
            *state = GuildShopState::Exited;
            GuildShopOutcome::SextantPurchased { price }
        }
        (GuildShopState::ConfirmSextant { .. }, GuildShopInput::Confirm(false)) => {
            *state = GuildShopState::Greeting;
            GuildShopOutcome::Declined
        }
        (GuildShopState::Exited, _) => GuildShopOutcome::Exited,
        _ => GuildShopOutcome::InvalidInput,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                adjusted_room_rate: 2,
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
            InnkeeperOutcome::PickLeaveCompanion { deposit: 30 }
        );

        let outcome = step_innkeeper(&mut state, InnkeeperInput::Slot(1), ctx);
        assert_eq!(
            outcome,
            InnkeeperOutcome::QuotedLeaveCompanion {
                party_index: 1,
                deposit: 30,
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
    fn reagent_shop_buy_full_path() {
        let mut state = ReagentShopState::Greeting;
        let mut gold = 1000u16;
        let mut stock = [0u8; 8];
        let prices = [5u16, 10, 15, 20, 25, 30, 50, 70];
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Key(b'Y'),
            &mut gold,
            &mut stock,
            &prices,
        );
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Reagent(2),
            &mut gold,
            &mut stock,
            &prices,
        );
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Quantity(4),
            &mut gold,
            &mut stock,
            &prices,
        );
        let outcome = step_reagent_shop(
            &mut state,
            ReagentShopInput::Confirm(true),
            &mut gold,
            &mut stock,
            &prices,
        );
        assert!(matches!(
            outcome,
            ReagentShopOutcome::Bought {
                reagent: 2,
                quantity: 4,
                paid: 60
            }
        ));
        assert_eq!(stock[2], 4);
        assert_eq!(gold, 1000 - 60);
    }

    #[test]
    fn reagent_shop_zero_quantity_treated_as_decline() {
        let mut state = ReagentShopState::Greeting;
        let mut gold = 1000u16;
        let mut stock = [0u8; 8];
        let prices = [5u16; 8];
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Key(b'Y'),
            &mut gold,
            &mut stock,
            &prices,
        );
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Reagent(0),
            &mut gold,
            &mut stock,
            &prices,
        );
        let outcome = step_reagent_shop(
            &mut state,
            ReagentShopInput::Quantity(0),
            &mut gold,
            &mut stock,
            &prices,
        );
        assert_eq!(outcome, ReagentShopOutcome::Declined);
        assert_eq!(gold, 1000);
    }

    #[test]
    fn reagent_shop_caps_stock_at_per_kind_max() {
        let mut state = ReagentShopState::Greeting;
        let mut gold = 60_000u16;
        let mut stock = [95u8; 8];
        let prices = [1u16; 8];
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Key(b'Y'),
            &mut gold,
            &mut stock,
            &prices,
        );
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Reagent(0),
            &mut gold,
            &mut stock,
            &prices,
        );
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Quantity(20),
            &mut gold,
            &mut stock,
            &prices,
        );
        step_reagent_shop(
            &mut state,
            ReagentShopInput::Confirm(true),
            &mut gold,
            &mut stock,
            &prices,
        );
        assert_eq!(stock[0], REAGENT_STOCK_CAP_PER_KIND);
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
            TavernInput::Key(b'P'),
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
            }
        );
    }

    #[test]
    fn horse_trader_purchase_path_marks_pending_delivery() {
        let mut state = HorseTraderState::Greeting;
        let mut gold = 500u16;
        let mut pending = false;
        step_horse_trader(
            &mut state,
            HorseTraderInput::Key(b'Y'),
            &mut gold,
            &mut pending,
        );
        let outcome = step_horse_trader(
            &mut state,
            HorseTraderInput::Confirm(true),
            &mut gold,
            &mut pending,
        );
        assert!(matches!(outcome, HorseTraderOutcome::Purchased { .. }));
        assert!(pending);
        assert_eq!(gold, 500 - HORSE_TRADER_DEFAULT_PRICE);
    }

    #[test]
    fn horse_trader_short_funds_refuses() {
        let mut state = HorseTraderState::Greeting;
        let mut gold = 10u16;
        let mut pending = false;
        step_horse_trader(
            &mut state,
            HorseTraderInput::Key(b'Y'),
            &mut gold,
            &mut pending,
        );
        let outcome = step_horse_trader(
            &mut state,
            HorseTraderInput::Confirm(true),
            &mut gold,
            &mut pending,
        );
        assert!(matches!(
            outcome,
            HorseTraderOutcome::RefusedShortFunds { .. }
        ));
        assert!(!pending);
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
            12,
            21,
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
            12,
            21,
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
                x: 12,
                y: 21,
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
            99,
            99,
        );
        let outcome = step_ship_broker(
            &mut state,
            ShipBrokerInput::Confirm(true),
            &mut gold,
            &mut pending,
            99,
            99,
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
    fn guild_shop_gems_path_debits_gold_and_increments_count() {
        let mut state = GuildShopState::Greeting;
        let mut gold = 1000u16;
        let mut gems = 0u8;
        let mut keys = 0u8;
        let mut torches = 0u8;
        let mut sextant = false;
        step_guild_shop(
            &mut state,
            GuildShopInput::Key(b'Y'),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        step_guild_shop(
            &mut state,
            GuildShopInput::Item(GuildItem::Gems),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        let outcome = step_guild_shop(
            &mut state,
            GuildShopInput::Quantity(3),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        assert!(matches!(
            outcome,
            GuildShopOutcome::Bought {
                item: GuildItem::Gems,
                quantity: 3,
                paid: 300
            }
        ));
        assert_eq!(gems, 3);
        assert_eq!(gold, 700);
    }

    #[test]
    fn guild_shop_sextant_only_sells_once() {
        let mut state = GuildShopState::Greeting;
        let mut gold = 5000u16;
        let mut gems = 0u8;
        let mut keys = 0u8;
        let mut torches = 0u8;
        let mut sextant = false;
        step_guild_shop(
            &mut state,
            GuildShopInput::Key(b'Y'),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        step_guild_shop(
            &mut state,
            GuildShopInput::Item(GuildItem::Sextant),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        let outcome = step_guild_shop(
            &mut state,
            GuildShopInput::Confirm(true),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        assert!(matches!(outcome, GuildShopOutcome::SextantPurchased { .. }));
        assert!(sextant);
        // Second attempt declines without charging — caller starts a
        // fresh visit since the previous one terminated.
        state = GuildShopState::Greeting;
        step_guild_shop(
            &mut state,
            GuildShopInput::Key(b'Y'),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        let outcome = step_guild_shop(
            &mut state,
            GuildShopInput::Item(GuildItem::Sextant),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        assert_eq!(outcome, GuildShopOutcome::Declined);
    }

    #[test]
    fn guild_shop_short_funds_refuses_quantity_purchase() {
        let mut state = GuildShopState::Greeting;
        let mut gold = 100u16;
        let mut gems = 0u8;
        let mut keys = 0u8;
        let mut torches = 0u8;
        let mut sextant = false;
        step_guild_shop(
            &mut state,
            GuildShopInput::Key(b'Y'),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        step_guild_shop(
            &mut state,
            GuildShopInput::Item(GuildItem::Gems),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        let outcome = step_guild_shop(
            &mut state,
            GuildShopInput::Quantity(5),
            &mut gold,
            &mut gems,
            &mut keys,
            &mut torches,
            &mut sextant,
        );
        assert!(matches!(
            outcome,
            GuildShopOutcome::RefusedShortFunds { .. }
        ));
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
        assert!(!healer_service_eligible(HealerService::Heal, dead));
        assert!(healer_service_eligible(HealerService::Resurrect, dead));
        assert!(!healer_service_eligible(
            HealerService::Resurrect,
            good_full
        ));
    }
}
