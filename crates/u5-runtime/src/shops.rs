//! Shop pricing and transaction helpers.

use crate::*;

pub const SHOP_COMMODITY_STOCK_CAP: u8 = 99;

/// `shops.md §6` arms-shop Buy quote. The shop's quote is the
/// canonical base price plus the integer-truncated Intelligence
/// adjustment `base * (100 - 3 * intelligence) / 100`. The same
/// item therefore quotes differently when a different party member
/// is speaking. Saturating math guards against absurd inputs.
pub const fn arms_shop_buy_quote(base_price: u16, speaker_intelligence: u8) -> u16 {
    let factor: i32 = 100 - 3 * speaker_intelligence as i32;
    let adjustment = (base_price as i32 * factor) / 100;
    let quoted = base_price as i32 + adjustment;
    if quoted < 0 {
        0
    } else if quoted > u16::MAX as i32 {
        u16::MAX
    } else {
        quoted as u16
    }
}

/// `shops.md §6` arms-shop Sell offer. The accepted item's offer is
/// `floor(base * 3 * intelligence / 100) + 1`. The party gold is
/// raised by the offer and the shared equipment counter is
/// decremented; this helper returns only the gold offer value.
pub const fn arms_shop_sell_offer(base_price: u16, speaker_intelligence: u8) -> u16 {
    let prod = base_price as u32 * 3 * speaker_intelligence as u32;
    let offer = prod / 100 + 1;
    if offer > u16::MAX as u32 { u16::MAX } else { offer as u16 }
}

/// `shops.md §4.1` `@` substitution time-of-day word the bark
/// renderer expands inline. The hour byte is read fresh from the
/// world clock on every render: hour < 12 → `morning`, hour < 18 →
/// `afternoon`, otherwise `evening`. Hours outside the 0..=23 clock
/// range fall through to the evening band.
pub const fn shoppe_time_of_day_word(hour: u8) -> &'static str {
    if hour < 12 {
        "morning"
    } else if hour < 18 {
        "afternoon"
    } else {
        "evening"
    }
}

/// `formats/shoppe-dat.md §2`: shipped DOS file size in bytes.
pub const SHOPPE_DAT_LEN: usize = 10_135;
/// `formats/shoppe-dat.md §2`: total record slots and non-empty
/// records in the shipped data set.
pub const SHOPPE_DAT_RECORD_SLOTS: usize = 196;
pub const SHOPPE_DAT_NONEMPTY_RECORDS: usize = 194;

/// `shops.md §4` per-cluster `SHOPPE.DAT` record-id ranges. Each
/// constant pair documents a shipped record cluster the per-shop-kind
/// tables hardcode. Some clusters intentionally overlap (sage rumour
/// records 84-91 sit inside the 57-88 tavern band) and a few slots
/// are unused NUL-only records.
pub const SHOPPE_RECORDS_SHARED_BARKS_FIRST: usize = 0;
pub const SHOPPE_RECORDS_SHARED_BARKS_LAST: usize = 7;

pub const SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST: usize = 8;
pub const SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST: usize = 48;

pub const SHOPPE_RECORDS_ARMS_SELL_FIRST: usize = 49;
pub const SHOPPE_RECORDS_ARMS_SELL_LAST: usize = 56;

pub const SHOPPE_RECORDS_TAVERN_FIRST: usize = 57;
pub const SHOPPE_RECORDS_TAVERN_LAST: usize = 88;

pub const SHOPPE_RECORDS_SAGE_FIRST: usize = 84;
pub const SHOPPE_RECORDS_SAGE_LAST: usize = 91;

pub const SHOPPE_RECORDS_HORSE_TRADER_FIRST: usize = 92;
pub const SHOPPE_RECORDS_HORSE_TRADER_LAST: usize = 104;

pub const SHOPPE_RECORDS_SHIP_BROKER_FIRST: usize = 105;
pub const SHOPPE_RECORDS_SHIP_BROKER_LAST: usize = 126;

pub const SHOPPE_RECORDS_REAGENT_FIRST: usize = 127;
pub const SHOPPE_RECORDS_REAGENT_LAST: usize = 146;

pub const SHOPPE_RECORDS_GUILD_FIRST: usize = 148;
pub const SHOPPE_RECORDS_GUILD_LAST: usize = 162;

pub const SHOPPE_RECORDS_HEALER_FIRST: usize = 163;
pub const SHOPPE_RECORDS_HEALER_LAST: usize = 173;

pub const SHOPPE_RECORDS_INNKEEPER_FIRST: usize = 174;
pub const SHOPPE_RECORDS_INNKEEPER_LAST: usize = 193;

/// `formats/shoppe-dat.md §4`: substitution placeholder a `SHOPPE.DAT`
/// renderer recognises in the literal-byte stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShoppePlaceholder {
    /// `%` — current gold amount, price, or total.
    Gold,
    /// `^` — current quantity.
    Quantity,
    /// `$` — vendor name.
    VendorName,
    /// `&` — item, subject, or asked-about name.
    ItemName,
    /// `*` — place or location name.
    PlaceName,
    /// `#` — shop name.
    ShopName,
    /// `@` — time-of-day word: morning/afternoon/evening.
    TimeOfDay,
}

/// `formats/shoppe-dat.md §4`: classify a literal byte into a
/// substitution placeholder. Returns `None` for ordinary ASCII bytes.
pub const fn shoppe_placeholder(byte: u8) -> Option<ShoppePlaceholder> {
    Some(match byte {
        b'%' => ShoppePlaceholder::Gold,
        b'^' => ShoppePlaceholder::Quantity,
        b'$' => ShoppePlaceholder::VendorName,
        b'&' => ShoppePlaceholder::ItemName,
        b'*' => ShoppePlaceholder::PlaceName,
        b'#' => ShoppePlaceholder::ShopName,
        b'@' => ShoppePlaceholder::TimeOfDay,
        _ => return None,
    })
}
pub const SHOP_FOOD_STOCK_CAP: u16 = 9999;
pub const SHOP_GOLD_CAP: u16 = 9999;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmsPurchaseQuote {
    pub item_id: usize,
    pub speaker_intelligence: u8,
    pub base_price: u16,
    pub total_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmsPurchaseOutcome {
    pub quote: ArmsPurchaseQuote,
    pub gold_before: u16,
    pub gold_after: u16,
    pub stock_before: u8,
    pub stock_after: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmsSaleQuote {
    pub item_id: usize,
    pub speaker_intelligence: u8,
    pub base_price: u16,
    pub offer: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmsSaleOutcome {
    pub quote: ArmsSaleQuote,
    pub gold_before: u16,
    pub gold_after: u16,
    pub stock_before: u8,
    pub stock_after: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShopSurchargeOutcome {
    pub sentinel: u8,
    pub surcharge: u16,
    pub gold_before: u16,
    pub gold_after: u16,
    pub applied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmsPurchaseError {
    InvalidItem,
    NotPurchasable,
    StockCap { current: u8, cap: u8 },
    InsufficientGold { available: u16, required: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmsSaleError {
    InvalidItem,
    NotSellable,
    NoStock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildShop {
    TheDen,
    TheGuild,
    TheNemesis,
}

impl GuildShop {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::TheDen => "The Den",
            Self::TheGuild => "The Guild",
            Self::TheNemesis => "The Nemesis",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildCommodity {
    Keys,
    Gems,
    Torches,
}

impl GuildCommodity {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Keys => "keys",
            Self::Gems => "gems",
            Self::Torches => "torches",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuildPurchaseQuote {
    pub shop: GuildShop,
    pub commodity: GuildCommodity,
    pub quantity: u8,
    pub unit_price: u16,
    pub total_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuildPurchaseOutcome {
    pub quote: GuildPurchaseQuote,
    pub gold_before: u16,
    pub gold_after: u16,
    pub stock_before: u8,
    pub stock_after: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildPurchaseError {
    ZeroQuantity,
    StockCap { current: u8, requested: u8, cap: u8 },
    InsufficientGold { available: u16, required: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shipwright {
    IslandShipwrights,
    TheCrowsNest,
    TheOakenOar,
    TheRustyBucket,
}

impl Shipwright {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::IslandShipwrights => "Island Shipwrights",
            Self::TheCrowsNest => "The Crow's Nest",
            Self::TheOakenOar => "The Oaken Oar",
            Self::TheRustyBucket => "The Rusty Bucket",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipwrightPurchaseKind {
    Frigate,
    Skiff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipwrightPurchaseQuote {
    pub shipwright: Shipwright,
    pub kind: ShipwrightPurchaseKind,
    pub price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipwrightPurchaseStatus {
    QueuedFrigate,
    QueuedSkiff,
    AddedSkiffToPendingFrigate,
    ExistingDeliveryRefusal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipwrightPurchaseOutcome {
    pub quote: ShipwrightPurchaseQuote,
    pub status: ShipwrightPurchaseStatus,
    pub gold_before: u16,
    pub gold_after: u16,
    pub pending_before: Option<PendingVehicleAcquisition>,
    pub pending_after: Option<PendingVehicleAcquisition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipwrightPurchaseError {
    InsufficientGold { available: u16, required: u16 },
    NoReturnWorld,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stable {
    HorseAndRider,
    TheStablehouse,
    WishingWellHorses,
}

impl Stable {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::HorseAndRider => "Horse & Rider",
            Self::TheStablehouse => "The Stablehouse",
            Self::WishingWellHorses => "Wishing Well Horses",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HorsePurchaseQuote {
    pub stable: Stable,
    pub price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HorsePurchaseOutcome {
    pub quote: HorsePurchaseQuote,
    pub gold_before: u16,
    pub gold_after: u16,
    pub active_object_slot: usize,
    pub horse: ActiveObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorsePurchaseError {
    InsufficientGold { available: u16, required: u16 },
    NoActiveObjectSlot,
    NoCurrentFloor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Healer {
    TheHealersMission,
    WoundsOfHonour,
    TheSpiritHealers,
    HealersSanctum,
    Sanctuary,
    TheShieldOfTruth,
    TheEmpath,
}

impl Healer {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::TheHealersMission => "The Healers Mission",
            Self::WoundsOfHonour => "Wounds of Honour",
            Self::TheSpiritHealers => "The Spirit Healers",
            Self::HealersSanctum => "Healers' Sanctum",
            Self::Sanctuary => "Sanctuary",
            Self::TheShieldOfTruth => "The Shield of Truth",
            Self::TheEmpath => "The Empath",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerTreatment {
    Cure,
    Heal,
    Resurrect,
}

impl HealerTreatment {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Cure => "Cure",
            Self::Heal => "Heal",
            Self::Resurrect => "Resurrect",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerTreatmentFee {
    Bypass,
    Price(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealerTreatmentQuote {
    pub healer: Healer,
    pub treatment: HealerTreatment,
    pub fee: HealerTreatmentFee,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealerTreatmentOutcome {
    pub quote: HealerTreatmentQuote,
    pub target_index: usize,
    pub gold_before: u16,
    pub gold_after: u16,
    pub status_before: u8,
    pub status_after: u8,
    pub hp_before: u16,
    pub hp_after: u16,
    pub max_hp_after: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerTreatmentError {
    InvalidTarget { party_len: usize, requested: usize },
    Untreatable,
    InsufficientGold { available: u16, required: u16 },
}

/// `shops.md §8.3` healer treatment eligibility per service letter.
/// Returns `true` when the healer should accept the selected target
/// for the requested treatment. Cure requires Poisoned status; Heal
/// refuses Dead members and members already at maximum HP (any
/// other status — including Poisoned — is eligible for the HP top-
/// up); Resurrect requires Dead status. Ashes and other non-Dead
/// statuses are never resurrected by the healer.
pub const fn healer_treatment_accepts(
    treatment: HealerTreatment,
    status: CharacterStatus,
    hp: u16,
    max_hp: u16,
) -> bool {
    match treatment {
        HealerTreatment::Cure => matches!(status, CharacterStatus::PoisonedOrRevived),
        HealerTreatment::Heal => {
            !matches!(status, CharacterStatus::Dead) && hp < max_hp
        }
        HealerTreatment::Resurrect => matches!(status, CharacterStatus::Dead),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Herbalist {
    TheHerbalist,
    HealersHerbs,
    TheAlchemist,
    Mysticism,
    TheSharperMage,
}

impl Herbalist {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::TheHerbalist => "The Herbalist",
            Self::HealersHerbs => "Healers Herbs",
            Self::TheAlchemist => "The Alchemist",
            Self::Mysticism => "Mysticism",
            Self::TheSharperMage => "The Sharper Mage",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reagent {
    SulfurAsh,
    Ginseng,
    Garlic,
    SpiderSilk,
    BloodMoss,
    BlackPearl,
    Nightshade,
    Mandrake,
}

impl Reagent {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::SulfurAsh => "Sulfur Ash",
            Self::Ginseng => "Ginseng",
            Self::Garlic => "Garlic",
            Self::SpiderSilk => "Spider Silk",
            Self::BloodMoss => "Blood Moss",
            Self::BlackPearl => "Black Pearl",
            Self::Nightshade => "Nightshade",
            Self::Mandrake => "Mandrake",
        }
    }

    pub const fn inventory_index(self) -> usize {
        match self {
            Self::SulfurAsh => REAGENT_SULFUR_ASH,
            Self::Ginseng => REAGENT_GINSENG,
            Self::Garlic => REAGENT_GARLIC,
            Self::SpiderSilk => REAGENT_SPIDER_SILK,
            Self::BloodMoss => REAGENT_BLOOD_MOSS,
            Self::BlackPearl => REAGENT_BLACK_PEARL,
            Self::Nightshade => REAGENT_NIGHTSHADE,
            Self::Mandrake => REAGENT_MANDRAKE,
        }
    }

    /// `magic.md §2`: short abbreviation used in the M-Mix prompt and
    /// other tight UI lines. Long names live in [`display_name`].
    pub const fn abbreviation(self) -> &'static str {
        match self {
            Self::SulfurAsh => "Sulfur Ash",
            Self::Ginseng => "Ginseng",
            Self::Garlic => "Garlic",
            Self::SpiderSilk => "Sp. Silk",
            Self::BloodMoss => "Blood Moss",
            Self::BlackPearl => "Blk. Pearl",
            Self::Nightshade => "Nightshade",
            Self::Mandrake => "Mandrake",
        }
    }
}

pub const REAGENT_VENDOR_ORDER: [Reagent; REAGENT_COUNT] = [
    Reagent::SulfurAsh,
    Reagent::Ginseng,
    Reagent::Garlic,
    Reagent::SpiderSilk,
    Reagent::BloodMoss,
    Reagent::BlackPearl,
    Reagent::Nightshade,
    Reagent::Mandrake,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReagentMenuEntry {
    pub letter: char,
    pub reagent: Reagent,
    pub unit_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReagentPurchaseQuote {
    pub herbalist: Herbalist,
    pub reagent: Reagent,
    pub quantity: u8,
    pub unit_price: u16,
    pub total_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReagentPurchaseOutcome {
    pub quote: ReagentPurchaseQuote,
    pub gold_before: u16,
    pub gold_after: u16,
    pub stock_before: u8,
    pub stock_after: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReagentPurchaseError {
    ZeroQuantity,
    NotStocked,
    StockCap { current: u8, requested: u8, cap: u8 },
    InsufficientGold { available: u16, required: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tavern {
    TheHonestMeal,
    TheWayfarerTavern,
    TheSwordAndKeg,
    TheSlaughteredLamb,
    TheHumblePalate,
    TheBlueBoarTavern,
    TheCatsLair,
    TheFallenVirgin,
    TheFolleyTap,
}

impl Tavern {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::TheHonestMeal => "The Honest Meal",
            Self::TheWayfarerTavern => "The Wayfarer Tavern",
            Self::TheSwordAndKeg => "The Sword and Keg",
            Self::TheSlaughteredLamb => "The Slaughtered Lamb",
            Self::TheHumblePalate => "The Humble Palate",
            Self::TheBlueBoarTavern => "The Blue Boar Tavern",
            Self::TheCatsLair => "The Cat's Lair",
            Self::TheFallenVirgin => "The Fallen Virgin",
            Self::TheFolleyTap => "The Folley Tap",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlueBoarDrinkChoice {
    A,
    B,
    C,
    D,
    E,
    F,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TavernRoundDrinkQuote {
    pub tavern: Tavern,
    pub menu_letter: char,
    pub living_party_members: u8,
    pub unit_price: u16,
    pub total_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TavernDrinkOutcome {
    pub gold_before: u16,
    pub gold_after: u16,
    pub total_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TavernDrinkError {
    NoLivingParty,
    InsufficientGold { available: u16, required: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProvisionPurchaseQuote {
    pub tavern: Tavern,
    pub quantity: u16,
    pub unit_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProvisionPurchaseOutcome {
    pub quote: ProvisionPurchaseQuote,
    pub requested_quantity: u16,
    pub purchased_quantity: u16,
    pub total_price: u16,
    pub gold_before: u16,
    pub gold_after: u16,
    pub food_before: u16,
    pub food_after: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvisionPurchaseError {
    ZeroQuantity,
    NoNeed,
    InsufficientGold {
        available: u16,
        required_per_unit: u16,
    },
}

pub const INN_REGISTRY_CAP: usize = 16;
pub const INN_PARTY_CAP: usize = 6;
pub const INN_STAY_COUNTER_CAP: u8 = 25;

/// `shops.md §8.4` Leave-companion deposit unit count. The deposit
/// debited when the player leaves a companion at an inn is the local
/// adjusted room-rate multiplied by this many units.
pub const INN_LEAVE_DEPOSIT_ROOM_RATE_UNITS: u8 = 10;

/// `shops.md §8.4`: Leave-companion deposit calculated from the
/// inn's adjusted room rate (already with Intelligence adjustment
/// applied). Returns the gold amount to debit before the registry
/// transfer completes.
pub const fn inn_leave_companion_deposit(adjusted_room_rate: u16) -> u16 {
    adjusted_room_rate * INN_LEAVE_DEPOSIT_ROOM_RATE_UNITS as u16
}

/// `shops.md §8.4`: Pickup bill calculated from the adjusted local
/// lodging charge and the guest's stored stay counter, treating zero
/// as one billable unit (so a same-day pickup still costs one
/// lodging charge).
pub const fn inn_pickup_bill(adjusted_lodging_charge: u16, stay_counter: u8) -> u16 {
    let units = if stay_counter == 0 { 1 } else { stay_counter };
    adjusted_lodging_charge * units as u16
}

/// `shops.md §8.4` morbid pickup conversion. A guest left at the
/// inn while Poisoned is converted to Dead on pickup: the returned
/// record's status flips to `'D'`, current HP is cleared, and the
/// inn prints "Thy friend has died, by the way." Other stored
/// statuses pass through unchanged.
pub const fn inn_pickup_status_converts_to_dead(stored_status: CharacterStatus) -> bool {
    matches!(stored_status, CharacterStatus::PoisonedOrRevived)
}

/// `shops.md §8.4` 28-day month-rollover stay-counter cap. Each
/// month rollover bumps the inn registry's per-guest stay counter
/// by one until this cap is reached; the pickup bill multiplies the
/// adjusted lodging charge by the stored counter.
pub const INN_STAY_COUNTER_MAX: u8 = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inn {
    TheWayfarerInn,
    TheWarriorsStead,
    TheHauntingInn,
    HotelBrittany,
    TheSmugglersInn,
    TheKingsRansomInn,
}

impl Inn {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::TheWayfarerInn => "The Wayfarer Inn",
            Self::TheWarriorsStead => "The Warrior's Stead",
            Self::TheHauntingInn => "The Haunting Inn",
            Self::HotelBrittany => "Hotel Brittany",
            Self::TheSmugglersInn => "The Smugglers' Inn",
            Self::TheKingsRansomInn => "The King's Ransom Inn",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnGuestRecord {
    pub scene_marker: u8,
    pub name: [u8; SAVE_CHARACTER_NAME_LEN],
    pub member: PartyMember,
    pub strength: u8,
    pub intelligence: u8,
    pub experience: u16,
    pub equipment: [u8; EQUIPMENT_SLOT_COUNT],
    pub stay_counter: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnRestQuote {
    pub inn: Inn,
    pub party_size: usize,
    pub adjusted_room_rate: u16,
    pub minimum_gold: u16,
    pub total_price: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnRestOutcome {
    pub quote: InnRestQuote,
    pub gold_before: u16,
    pub gold_after: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnLeaveOutcome {
    pub scene_marker: u8,
    pub party_index: usize,
    pub registry_index: usize,
    pub deposit: u16,
    pub gold_before: u16,
    pub gold_after: u16,
    pub guest: InnGuestRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnPickupOutcome {
    pub scene_marker: u8,
    pub registry_index: usize,
    pub party_index: usize,
    pub billable_stay_units: u8,
    pub bill: u16,
    pub gold_before: u16,
    pub gold_after: u16,
    pub returned_dead_from_poison: bool,
    pub guest: InnGuestRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InnError {
    EmptyParty,
    PartyTooSmallToLeave,
    PartyFull,
    InvalidPartyIndex {
        party_len: usize,
        requested: usize,
    },
    InvalidGuestIndex {
        registry_len: usize,
        requested: usize,
    },
    GuestNotAtInn {
        scene_marker: u8,
        requested_scene: u8,
    },
    RegistryFull,
    BelowMinimumGold {
        available: u16,
        minimum: u16,
    },
    InsufficientGold {
        available: u16,
        required: u16,
    },
}

pub const SAGE_TOPIC_INPUT_LIMIT: usize = 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SageTopic {
    pub topic: &'static str,
    pub subject: &'static str,
    pub destination: &'static str,
    pub fee: u16,
    pub template: SageRumourTemplate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SageRumourTemplate {
    SeekSubjectInDestination,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SageRumourQuote {
    pub topic: SageTopic,
    pub input_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SageRumourOutcome {
    pub quote: SageRumourQuote,
    pub gold_before: u16,
    pub gold_after: u16,
    pub rendered: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SageRumourError {
    EmptyInput,
    InputTooLong { limit: usize, actual: usize },
    NoTopicMatch,
    InsufficientGold { available: u16, required: u16 },
}

pub fn quote_arms_purchase(
    item_id: usize,
    speaker_intelligence: u8,
) -> Result<ArmsPurchaseQuote, ArmsPurchaseError> {
    let Some(base_price) = equipment_base_price(item_id) else {
        return Err(ArmsPurchaseError::InvalidItem);
    };
    if base_price == 0 {
        return Err(ArmsPurchaseError::NotPurchasable);
    }

    let intelligence = speaker_intelligence.min(AVATAR_STAT_MAX);
    let adjustment = base_price * (100 - 3 * intelligence as u16) / 100;
    Ok(ArmsPurchaseQuote {
        item_id,
        speaker_intelligence: intelligence,
        base_price,
        total_price: base_price + adjustment,
    })
}

pub fn apply_arms_purchase(
    gold: &mut u16,
    stock: &mut u8,
    item_id: usize,
    speaker_intelligence: u8,
) -> Result<ArmsPurchaseOutcome, ArmsPurchaseError> {
    let quote = quote_arms_purchase(item_id, speaker_intelligence)?;
    if *stock >= EQUIPMENT_STOCK_CAP {
        return Err(ArmsPurchaseError::StockCap {
            current: *stock,
            cap: EQUIPMENT_STOCK_CAP,
        });
    }
    if *gold < quote.total_price {
        return Err(ArmsPurchaseError::InsufficientGold {
            available: *gold,
            required: quote.total_price,
        });
    }

    let gold_before = *gold;
    let stock_before = *stock;
    *gold -= quote.total_price;
    *stock += 1;

    Ok(ArmsPurchaseOutcome {
        quote,
        gold_before,
        gold_after: *gold,
        stock_before,
        stock_after: *stock,
    })
}

pub fn quote_arms_sale(
    item_id: usize,
    speaker_intelligence: u8,
) -> Result<ArmsSaleQuote, ArmsSaleError> {
    let Some(base_price) = equipment_base_price(item_id) else {
        return Err(ArmsSaleError::InvalidItem);
    };
    if base_price == 0 || matches!(item_id, EQUIPMENT_ID_ARROWS | EQUIPMENT_ID_QUARRELS) {
        return Err(ArmsSaleError::NotSellable);
    }

    let intelligence = speaker_intelligence.min(AVATAR_STAT_MAX);
    Ok(ArmsSaleQuote {
        item_id,
        speaker_intelligence: intelligence,
        base_price,
        offer: base_price * 3 * intelligence as u16 / 100 + 1,
    })
}

pub fn apply_arms_sale(
    gold: &mut u16,
    stock: &mut u8,
    item_id: usize,
    speaker_intelligence: u8,
) -> Result<ArmsSaleOutcome, ArmsSaleError> {
    let quote = quote_arms_sale(item_id, speaker_intelligence)?;
    if *stock == 0 {
        return Err(ArmsSaleError::NoStock);
    }

    let gold_before = *gold;
    let stock_before = *stock;
    *gold = (*gold).saturating_add(quote.offer).min(SHOP_GOLD_CAP);
    *stock -= 1;

    Ok(ArmsSaleOutcome {
        quote,
        gold_before,
        gold_after: *gold,
        stock_before,
        stock_after: *stock,
    })
}

pub const fn shop_surcharge_from_roll_seed(roll_seed: u8) -> u16 {
    1 + (roll_seed & 0x3f) as u16
}

pub fn apply_shop_surcharge(
    gold: &mut u16,
    shared_town_conversation_sentinel: u8,
    roll_seed: u8,
) -> ShopSurchargeOutcome {
    let surcharge = shop_surcharge_from_roll_seed(roll_seed);
    let gold_before = *gold;
    let applied = shared_town_conversation_sentinel == 0;
    if applied {
        *gold = (*gold).saturating_sub(surcharge);
    }

    ShopSurchargeOutcome {
        sentinel: shared_town_conversation_sentinel,
        surcharge,
        gold_before,
        gold_after: *gold,
        applied,
    }
}

pub const fn guild_unit_price(shop: GuildShop, commodity: GuildCommodity) -> u16 {
    match shop {
        GuildShop::TheDen => match commodity {
            GuildCommodity::Keys => 190,
            GuildCommodity::Gems => 255,
            GuildCommodity::Torches => 12,
        },
        GuildShop::TheGuild => match commodity {
            GuildCommodity::Keys => 160,
            GuildCommodity::Gems => 200,
            GuildCommodity::Torches => 11,
        },
        GuildShop::TheNemesis => match commodity {
            GuildCommodity::Keys => 185,
            GuildCommodity::Gems => 225,
            GuildCommodity::Torches => 25,
        },
    }
}

pub const fn herbalist_reagent_price(herbalist: Herbalist, reagent: Reagent) -> Option<u16> {
    match herbalist {
        Herbalist::TheHerbalist => match reagent {
            Reagent::Ginseng => Some(20),
            Reagent::Garlic => Some(18),
            Reagent::SpiderSilk => Some(12),
            Reagent::Nightshade => Some(12),
            Reagent::Mandrake => Some(13),
            _ => None,
        },
        Herbalist::HealersHerbs => match reagent {
            Reagent::SulfurAsh => Some(12),
            Reagent::Ginseng => Some(16),
            Reagent::Garlic => Some(16),
            Reagent::SpiderSilk => Some(8),
            Reagent::BloodMoss => Some(20),
            _ => None,
        },
        Herbalist::TheAlchemist => match reagent {
            Reagent::SulfurAsh => Some(14),
            Reagent::Ginseng => Some(16),
            Reagent::BloodMoss => Some(30),
            Reagent::BlackPearl => Some(18),
            _ => None,
        },
        Herbalist::Mysticism => match reagent {
            Reagent::SpiderSilk => Some(6),
            Reagent::BloodMoss => Some(8),
            Reagent::BlackPearl => Some(8),
            Reagent::Nightshade => Some(10),
            Reagent::Mandrake => Some(15),
            _ => None,
        },
        Herbalist::TheSharperMage => match reagent {
            Reagent::BloodMoss => Some(50),
            Reagent::Nightshade => Some(30),
            Reagent::Mandrake => Some(40),
            _ => None,
        },
    }
}

pub fn herbalist_menu_entries(herbalist: Herbalist) -> Vec<ReagentMenuEntry> {
    let mut entries = Vec::new();
    for reagent in REAGENT_VENDOR_ORDER {
        if let Some(unit_price) = herbalist_reagent_price(herbalist, reagent) {
            let letter = (b'A' + entries.len() as u8) as char;
            entries.push(ReagentMenuEntry {
                letter,
                reagent,
                unit_price,
            });
        }
    }
    entries
}

pub const fn quote_reagent_purchase(
    herbalist: Herbalist,
    reagent: Reagent,
    quantity: u8,
) -> Result<ReagentPurchaseQuote, ReagentPurchaseError> {
    if quantity == 0 {
        return Err(ReagentPurchaseError::ZeroQuantity);
    }
    let Some(unit_price) = herbalist_reagent_price(herbalist, reagent) else {
        return Err(ReagentPurchaseError::NotStocked);
    };
    Ok(ReagentPurchaseQuote {
        herbalist,
        reagent,
        quantity,
        unit_price,
        total_price: unit_price * quantity as u16,
    })
}

pub fn apply_reagent_purchase(
    gold: &mut u16,
    stock: &mut u8,
    herbalist: Herbalist,
    reagent: Reagent,
    quantity: u8,
) -> Result<ReagentPurchaseOutcome, ReagentPurchaseError> {
    let quote = quote_reagent_purchase(herbalist, reagent, quantity)?;
    let Some(stock_after) = stock.checked_add(quantity) else {
        return Err(ReagentPurchaseError::StockCap {
            current: *stock,
            requested: quantity,
            cap: SHOP_COMMODITY_STOCK_CAP,
        });
    };
    if stock_after > SHOP_COMMODITY_STOCK_CAP {
        return Err(ReagentPurchaseError::StockCap {
            current: *stock,
            requested: quantity,
            cap: SHOP_COMMODITY_STOCK_CAP,
        });
    }
    if *gold < quote.total_price {
        return Err(ReagentPurchaseError::InsufficientGold {
            available: *gold,
            required: quote.total_price,
        });
    }

    let gold_before = *gold;
    let stock_before = *stock;
    *gold -= quote.total_price;
    *stock = stock_after;

    Ok(ReagentPurchaseOutcome {
        quote,
        gold_before,
        gold_after: *gold,
        stock_before,
        stock_after,
    })
}

pub const fn tavern_provision_unit_price(tavern: Tavern) -> u16 {
    match tavern {
        Tavern::TheHonestMeal => 10,
        Tavern::TheWayfarerTavern => 15,
        Tavern::TheSwordAndKeg => 20,
        Tavern::TheSlaughteredLamb => 25,
        Tavern::TheHumblePalate => 30,
        Tavern::TheBlueBoarTavern => 25,
        Tavern::TheCatsLair => 20,
        Tavern::TheFallenVirgin => 25,
        Tavern::TheFolleyTap => 30,
    }
}

pub const fn tavern_round_drink_menu_letter(tavern: Tavern) -> char {
    match tavern {
        Tavern::TheHonestMeal => 'M',
        Tavern::TheWayfarerTavern => 'M',
        Tavern::TheSwordAndKeg => 'M',
        Tavern::TheSlaughteredLamb => 'B',
        Tavern::TheHumblePalate => 'F',
        Tavern::TheBlueBoarTavern => 'C',
        Tavern::TheCatsLair => 'M',
        Tavern::TheFallenVirgin => 'B',
        Tavern::TheFolleyTap => 'M',
    }
}

pub const fn tavern_round_drink_unit_price(tavern: Tavern) -> u16 {
    match tavern {
        Tavern::TheHonestMeal => 3,
        Tavern::TheWayfarerTavern => 4,
        Tavern::TheSwordAndKeg => 5,
        Tavern::TheSlaughteredLamb => 3,
        Tavern::TheHumblePalate => 2,
        Tavern::TheBlueBoarTavern => 5,
        Tavern::TheCatsLair => 3,
        Tavern::TheFallenVirgin => 4,
        Tavern::TheFolleyTap => 5,
    }
}

pub const fn blue_boar_drink_price(choice: BlueBoarDrinkChoice) -> u16 {
    match choice {
        BlueBoarDrinkChoice::A => 18,
        BlueBoarDrinkChoice::B => 192,
        BlueBoarDrinkChoice::C => 79,
        BlueBoarDrinkChoice::D => 30,
        BlueBoarDrinkChoice::E => 275,
        BlueBoarDrinkChoice::F => 98,
    }
}

pub const fn quote_tavern_round_drink(
    tavern: Tavern,
    living_party_members: u8,
) -> Result<TavernRoundDrinkQuote, TavernDrinkError> {
    if living_party_members == 0 {
        return Err(TavernDrinkError::NoLivingParty);
    }
    let unit_price = tavern_round_drink_unit_price(tavern);
    Ok(TavernRoundDrinkQuote {
        tavern,
        menu_letter: tavern_round_drink_menu_letter(tavern),
        living_party_members,
        unit_price,
        total_price: unit_price * living_party_members as u16,
    })
}

pub fn apply_tavern_round_drink(
    gold: &mut u16,
    tavern: Tavern,
    living_party_members: u8,
) -> Result<TavernDrinkOutcome, TavernDrinkError> {
    let quote = quote_tavern_round_drink(tavern, living_party_members)?;
    if *gold < quote.total_price {
        return Err(TavernDrinkError::InsufficientGold {
            available: *gold,
            required: quote.total_price,
        });
    }

    let gold_before = *gold;
    *gold -= quote.total_price;
    Ok(TavernDrinkOutcome {
        gold_before,
        gold_after: *gold,
        total_price: quote.total_price,
    })
}

pub fn apply_blue_boar_drink(
    gold: &mut u16,
    choice: BlueBoarDrinkChoice,
) -> Result<TavernDrinkOutcome, TavernDrinkError> {
    let price = blue_boar_drink_price(choice);
    if *gold < price {
        return Err(TavernDrinkError::InsufficientGold {
            available: *gold,
            required: price,
        });
    }

    let gold_before = *gold;
    *gold -= price;
    Ok(TavernDrinkOutcome {
        gold_before,
        gold_after: *gold,
        total_price: price,
    })
}

pub const fn quote_provision_purchase(
    tavern: Tavern,
    quantity: u16,
) -> Result<ProvisionPurchaseQuote, ProvisionPurchaseError> {
    if quantity == 0 {
        return Err(ProvisionPurchaseError::ZeroQuantity);
    }
    Ok(ProvisionPurchaseQuote {
        tavern,
        quantity,
        unit_price: tavern_provision_unit_price(tavern),
    })
}

pub fn apply_provision_purchase(
    gold: &mut u16,
    food: &mut u16,
    tavern: Tavern,
    quantity: u16,
) -> Result<ProvisionPurchaseOutcome, ProvisionPurchaseError> {
    let quote = quote_provision_purchase(tavern, quantity)?;
    if *food >= SHOP_FOOD_STOCK_CAP {
        return Err(ProvisionPurchaseError::NoNeed);
    }
    if *gold < quote.unit_price {
        return Err(ProvisionPurchaseError::InsufficientGold {
            available: *gold,
            required_per_unit: quote.unit_price,
        });
    }

    let affordable_units = *gold / quote.unit_price;
    let food_capacity = SHOP_FOOD_STOCK_CAP - *food;
    let purchased_quantity = quantity.min(affordable_units).min(food_capacity);
    let total_price = purchased_quantity * quote.unit_price;
    let gold_before = *gold;
    let food_before = *food;
    *gold -= total_price;
    *food += purchased_quantity;

    Ok(ProvisionPurchaseOutcome {
        quote,
        requested_quantity: quantity,
        purchased_quantity,
        total_price,
        gold_before,
        gold_after: *gold,
        food_before,
        food_after: *food,
    })
}

pub const fn inn_base_room_rate(inn: Inn) -> u16 {
    match inn {
        Inn::TheWayfarerInn => 2,
        Inn::TheWarriorsStead => 3,
        Inn::TheHauntingInn => 2,
        Inn::HotelBrittany => 3,
        Inn::TheSmugglersInn => 2,
        Inn::TheKingsRansomInn => 3,
    }
}

pub const fn inn_minimum_gold(inn: Inn) -> u16 {
    match inn {
        Inn::TheWayfarerInn => 3,
        Inn::TheWarriorsStead => 4,
        Inn::TheHauntingInn => 3,
        Inn::HotelBrittany => 2,
        Inn::TheSmugglersInn => 2,
        Inn::TheKingsRansomInn => 2,
    }
}

pub fn inn_guest_indices_for_scene(registry: &[InnGuestRecord], scene_marker: u8) -> Vec<usize> {
    registry
        .iter()
        .enumerate()
        .filter_map(|(index, guest)| (guest.scene_marker == scene_marker).then_some(index))
        .collect()
}

pub const fn inn_billable_stay_units(stay_counter: u8) -> u8 {
    if stay_counter == 0 {
        1
    } else if stay_counter > INN_STAY_COUNTER_CAP {
        INN_STAY_COUNTER_CAP
    } else {
        stay_counter
    }
}

pub fn age_inn_registry_month(registry: &mut [InnGuestRecord]) -> usize {
    let mut aged = 0;
    for guest in registry {
        let previous = guest.stay_counter;
        guest.stay_counter = guest
            .stay_counter
            .saturating_add(1)
            .min(INN_STAY_COUNTER_CAP);
        if guest.stay_counter != previous {
            aged += 1;
        }
    }
    aged
}

pub fn age_stay_counters_month(stay_counters: &mut [u8]) -> usize {
    let mut aged = 0;
    for stay_counter in stay_counters {
        let previous = *stay_counter;
        *stay_counter = (*stay_counter).saturating_add(1).min(INN_STAY_COUNTER_CAP);
        if *stay_counter != previous {
            aged += 1;
        }
    }
    aged
}

pub fn quote_inn_rest(
    inn: Inn,
    party_size: usize,
    adjusted_room_rate: u16,
) -> Result<InnRestQuote, InnError> {
    if party_size == 0 {
        return Err(InnError::EmptyParty);
    }
    Ok(InnRestQuote {
        inn,
        party_size,
        adjusted_room_rate,
        minimum_gold: inn_minimum_gold(inn),
        total_price: adjusted_room_rate * party_size as u16,
    })
}

pub fn apply_inn_rest_payment(
    gold: &mut u16,
    inn: Inn,
    party_size: usize,
    adjusted_room_rate: u16,
) -> Result<InnRestOutcome, InnError> {
    let quote = quote_inn_rest(inn, party_size, adjusted_room_rate)?;
    if *gold < quote.minimum_gold {
        return Err(InnError::BelowMinimumGold {
            available: *gold,
            minimum: quote.minimum_gold,
        });
    }
    if *gold < quote.total_price {
        return Err(InnError::InsufficientGold {
            available: *gold,
            required: quote.total_price,
        });
    }

    let gold_before = *gold;
    *gold -= quote.total_price;
    Ok(InnRestOutcome {
        quote,
        gold_before,
        gold_after: *gold,
    })
}

pub fn apply_inn_leave_guest(
    gold: &mut u16,
    registry: &mut Vec<InnGuestRecord>,
    scene_marker: u8,
    party: &mut Vec<PartyMember>,
    party_names: &mut Vec<[u8; SAVE_CHARACTER_NAME_LEN]>,
    party_stay_counters: &mut Vec<u8>,
    party_strengths: &mut Vec<u8>,
    party_intelligence: &mut Vec<u8>,
    party_experience: &mut Vec<u16>,
    party_equipment: &mut Vec<[u8; EQUIPMENT_SLOT_COUNT]>,
    party_index: usize,
    adjusted_lodging_charge: u16,
) -> Result<InnLeaveOutcome, InnError> {
    if party.len() <= 1 {
        return Err(InnError::PartyTooSmallToLeave);
    }
    if party_index >= party.len() {
        return Err(InnError::InvalidPartyIndex {
            party_len: party.len(),
            requested: party_index,
        });
    }
    if registry.len() >= INN_REGISTRY_CAP {
        return Err(InnError::RegistryFull);
    }
    if *gold < adjusted_lodging_charge {
        return Err(InnError::InsufficientGold {
            available: *gold,
            required: adjusted_lodging_charge,
        });
    }

    let member = party.remove(party_index);
    let name = remove_or_default(party_names, party_index, [0; SAVE_CHARACTER_NAME_LEN]);
    let _stay_counter = remove_or_default(party_stay_counters, party_index, 0);
    let strength = remove_or_default(party_strengths, party_index, AVATAR_STAT_MAX);
    let intelligence = remove_or_default(party_intelligence, party_index, AVATAR_STAT_MAX);
    let experience = remove_or_default(party_experience, party_index, 0);
    let equipment = remove_or_default(
        party_equipment,
        party_index,
        [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
    );
    normalize_party_slots(party);

    let guest = InnGuestRecord {
        scene_marker,
        name,
        member,
        strength,
        intelligence,
        experience,
        equipment,
        stay_counter: 0,
    };
    let gold_before = *gold;
    *gold -= adjusted_lodging_charge;
    registry.push(guest);
    Ok(InnLeaveOutcome {
        scene_marker,
        party_index,
        registry_index: registry.len() - 1,
        deposit: adjusted_lodging_charge,
        gold_before,
        gold_after: *gold,
        guest,
    })
}

pub fn apply_inn_pickup_guest(
    gold: &mut u16,
    registry: &mut Vec<InnGuestRecord>,
    scene_marker: u8,
    party: &mut Vec<PartyMember>,
    party_names: &mut Vec<[u8; SAVE_CHARACTER_NAME_LEN]>,
    party_stay_counters: &mut Vec<u8>,
    party_strengths: &mut Vec<u8>,
    party_intelligence: &mut Vec<u8>,
    party_experience: &mut Vec<u16>,
    party_equipment: &mut Vec<[u8; EQUIPMENT_SLOT_COUNT]>,
    registry_index: usize,
    adjusted_lodging_charge: u16,
) -> Result<InnPickupOutcome, InnError> {
    if party.len() >= INN_PARTY_CAP {
        return Err(InnError::PartyFull);
    }
    let Some(guest) = registry.get(registry_index).copied() else {
        return Err(InnError::InvalidGuestIndex {
            registry_len: registry.len(),
            requested: registry_index,
        });
    };
    if guest.scene_marker != scene_marker {
        return Err(InnError::GuestNotAtInn {
            scene_marker: guest.scene_marker,
            requested_scene: scene_marker,
        });
    }

    let billable_stay_units = inn_billable_stay_units(guest.stay_counter);
    let bill = adjusted_lodging_charge * u16::from(billable_stay_units);
    if *gold < bill {
        return Err(InnError::InsufficientGold {
            available: *gold,
            required: bill,
        });
    }

    let mut member = guest.member;
    let returned_dead_from_poison = member.status == b'P';
    if returned_dead_from_poison {
        member.status = b'D';
        member.hp = 0;
    }
    member.slot = party.len() as u8;
    let party_index = party.len();

    let gold_before = *gold;
    *gold -= bill;
    registry.remove(registry_index);
    party.push(member);
    party_names.push(guest.name);
    party_stay_counters.push(guest.stay_counter.min(INN_STAY_COUNTER_CAP));
    party_strengths.push(guest.strength);
    party_intelligence.push(guest.intelligence);
    party_experience.push(guest.experience);
    party_equipment.push(guest.equipment);

    Ok(InnPickupOutcome {
        scene_marker,
        registry_index,
        party_index,
        billable_stay_units,
        bill,
        gold_before,
        gold_after: *gold,
        returned_dead_from_poison,
        guest,
    })
}

fn remove_or_default<T: Copy>(values: &mut Vec<T>, index: usize, default: T) -> T {
    if index < values.len() {
        values.remove(index)
    } else {
        default
    }
}

fn normalize_party_slots(party: &mut [PartyMember]) {
    for (index, member) in party.iter_mut().enumerate() {
        member.slot = index as u8;
    }
}

pub fn sage_topic_matches_input(topic: &str, input: &str) -> bool {
    let input = input.trim_start();
    if input.len() < topic.len() {
        return false;
    }
    let Some(candidate) = input.get(..topic.len()) else {
        return false;
    };
    if !candidate.eq_ignore_ascii_case(topic) {
        return false;
    }
    matches!(input.as_bytes().get(topic.len()), None | Some(b' '))
}

pub fn find_sage_topic(
    topics: &[SageTopic],
    input: &str,
) -> Result<SageRumourQuote, SageRumourError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(SageRumourError::EmptyInput);
    }
    if trimmed.len() > SAGE_TOPIC_INPUT_LIMIT {
        return Err(SageRumourError::InputTooLong {
            limit: SAGE_TOPIC_INPUT_LIMIT,
            actual: trimmed.len(),
        });
    }

    topics
        .iter()
        .copied()
        .find(|topic| sage_topic_matches_input(topic.topic, trimmed))
        .map(|topic| SageRumourQuote {
            topic,
            input_len: trimmed.len(),
        })
        .ok_or(SageRumourError::NoTopicMatch)
}

pub fn render_sage_rumour(topic: SageTopic) -> String {
    match topic.template {
        SageRumourTemplate::SeekSubjectInDestination => {
            format!("Seek ye {} in {}!", topic.subject, topic.destination)
        }
    }
}

pub fn apply_sage_rumour_purchase(
    gold: &mut u16,
    topics: &[SageTopic],
    input: &str,
) -> Result<SageRumourOutcome, SageRumourError> {
    let quote = find_sage_topic(topics, input)?;
    if *gold < quote.topic.fee {
        return Err(SageRumourError::InsufficientGold {
            available: *gold,
            required: quote.topic.fee,
        });
    }

    let gold_before = *gold;
    *gold -= quote.topic.fee;
    Ok(SageRumourOutcome {
        quote,
        gold_before,
        gold_after: *gold,
        rendered: render_sage_rumour(quote.topic),
    })
}

pub const fn healer_treatment_fee(
    healer: Healer,
    treatment: HealerTreatment,
) -> HealerTreatmentFee {
    match healer {
        Healer::TheHealersMission => match treatment {
            HealerTreatment::Cure | HealerTreatment::Heal => HealerTreatmentFee::Bypass,
            HealerTreatment::Resurrect => HealerTreatmentFee::Price(200),
        },
        Healer::WoundsOfHonour => match treatment {
            HealerTreatment::Cure => HealerTreatmentFee::Price(25),
            HealerTreatment::Heal => HealerTreatmentFee::Price(40),
            HealerTreatment::Resurrect => HealerTreatmentFee::Price(215),
        },
        Healer::TheSpiritHealers => match treatment {
            HealerTreatment::Cure => HealerTreatmentFee::Price(30),
            HealerTreatment::Heal => HealerTreatmentFee::Price(45),
            HealerTreatment::Resurrect => HealerTreatmentFee::Price(225),
        },
        Healer::HealersSanctum => match treatment {
            HealerTreatment::Cure => HealerTreatmentFee::Price(35),
            HealerTreatment::Heal => HealerTreatmentFee::Price(50),
            HealerTreatment::Resurrect => HealerTreatmentFee::Price(237),
        },
        Healer::Sanctuary => match treatment {
            HealerTreatment::Cure => HealerTreatmentFee::Price(40),
            HealerTreatment::Heal => HealerTreatmentFee::Price(55),
            HealerTreatment::Resurrect => HealerTreatmentFee::Price(247),
        },
        Healer::TheShieldOfTruth => match treatment {
            HealerTreatment::Cure => HealerTreatmentFee::Price(15),
            HealerTreatment::Heal => HealerTreatmentFee::Price(60),
            HealerTreatment::Resurrect => HealerTreatmentFee::Price(249),
        },
        Healer::TheEmpath => match treatment {
            HealerTreatment::Cure => HealerTreatmentFee::Price(10),
            HealerTreatment::Heal => HealerTreatmentFee::Price(65),
            HealerTreatment::Resurrect => HealerTreatmentFee::Price(262),
        },
    }
}

pub const fn quote_healer_treatment(
    healer: Healer,
    treatment: HealerTreatment,
) -> HealerTreatmentQuote {
    HealerTreatmentQuote {
        healer,
        treatment,
        fee: healer_treatment_fee(healer, treatment),
    }
}

pub const fn stable_horse_price(stable: Stable) -> u16 {
    match stable {
        Stable::HorseAndRider => 100,
        Stable::TheStablehouse => 130,
        Stable::WishingWellHorses => 160,
    }
}

pub const fn quote_horse_purchase(stable: Stable) -> HorsePurchaseQuote {
    HorsePurchaseQuote {
        stable,
        price: stable_horse_price(stable),
    }
}

pub const fn horse_purchase_active_object(x: usize, y: usize, z: i8) -> ActiveObject {
    ActiveObject {
        type_byte: FIRST_PLAYABLE_HORSE_TILE,
        tile: FIRST_PLAYABLE_HORSE_TILE,
        x,
        y,
        z,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    }
}

pub const fn shipwright_price(shipwright: Shipwright, kind: ShipwrightPurchaseKind) -> u16 {
    match shipwright {
        Shipwright::IslandShipwrights => match kind {
            ShipwrightPurchaseKind::Frigate => 600,
            ShipwrightPurchaseKind::Skiff => 200,
        },
        Shipwright::TheCrowsNest => match kind {
            ShipwrightPurchaseKind::Frigate => 753,
            ShipwrightPurchaseKind::Skiff => 175,
        },
        Shipwright::TheOakenOar => match kind {
            ShipwrightPurchaseKind::Frigate => 650,
            ShipwrightPurchaseKind::Skiff => 125,
        },
        Shipwright::TheRustyBucket => match kind {
            ShipwrightPurchaseKind::Frigate => 700,
            ShipwrightPurchaseKind::Skiff => 100,
        },
    }
}

pub const fn quote_shipwright_purchase(
    shipwright: Shipwright,
    kind: ShipwrightPurchaseKind,
) -> ShipwrightPurchaseQuote {
    ShipwrightPurchaseQuote {
        shipwright,
        kind,
        price: shipwright_price(shipwright, kind),
    }
}

pub fn apply_shipwright_purchase(
    gold: &mut u16,
    pending_vehicle: &mut Option<PendingVehicleAcquisition>,
    shipwright: Shipwright,
    kind: ShipwrightPurchaseKind,
    delivery_x: usize,
    delivery_y: usize,
) -> Result<ShipwrightPurchaseOutcome, ShipwrightPurchaseError> {
    let quote = quote_shipwright_purchase(shipwright, kind);
    let pending_before = *pending_vehicle;

    let (status, pending_after) = match (kind, pending_before) {
        (ShipwrightPurchaseKind::Frigate, None) => (
            ShipwrightPurchaseStatus::QueuedFrigate,
            Some(PendingVehicleAcquisition::Frigate {
                x: delivery_x,
                y: delivery_y,
                skiffs: 2,
            }),
        ),
        (ShipwrightPurchaseKind::Frigate, Some(_)) => (
            ShipwrightPurchaseStatus::ExistingDeliveryRefusal,
            pending_before,
        ),
        (
            ShipwrightPurchaseKind::Skiff,
            Some(PendingVehicleAcquisition::Frigate { x, y, skiffs }),
        ) => (
            ShipwrightPurchaseStatus::AddedSkiffToPendingFrigate,
            Some(PendingVehicleAcquisition::Frigate {
                x,
                y,
                skiffs: skiffs.saturating_add(1),
            }),
        ),
        (ShipwrightPurchaseKind::Skiff, Some(PendingVehicleAcquisition::Skiff { .. })) => (
            ShipwrightPurchaseStatus::ExistingDeliveryRefusal,
            pending_before,
        ),
        (ShipwrightPurchaseKind::Skiff, None) => (
            ShipwrightPurchaseStatus::QueuedSkiff,
            Some(PendingVehicleAcquisition::Skiff {
                x: delivery_x,
                y: delivery_y,
            }),
        ),
    };

    if status == ShipwrightPurchaseStatus::ExistingDeliveryRefusal {
        if kind == ShipwrightPurchaseKind::Frigate && *gold < quote.price {
            return Err(ShipwrightPurchaseError::InsufficientGold {
                available: *gold,
                required: quote.price,
            });
        }
        return Ok(ShipwrightPurchaseOutcome {
            quote,
            status,
            gold_before: *gold,
            gold_after: *gold,
            pending_before,
            pending_after,
        });
    }

    if *gold < quote.price {
        return Err(ShipwrightPurchaseError::InsufficientGold {
            available: *gold,
            required: quote.price,
        });
    }

    let gold_before = *gold;
    *gold -= quote.price;
    *pending_vehicle = pending_after;

    Ok(ShipwrightPurchaseOutcome {
        quote,
        status,
        gold_before,
        gold_after: *gold,
        pending_before,
        pending_after,
    })
}

pub const fn quote_guild_purchase(
    shop: GuildShop,
    commodity: GuildCommodity,
    quantity: u8,
) -> Result<GuildPurchaseQuote, GuildPurchaseError> {
    if quantity == 0 {
        return Err(GuildPurchaseError::ZeroQuantity);
    }

    let unit_price = guild_unit_price(shop, commodity);
    Ok(GuildPurchaseQuote {
        shop,
        commodity,
        quantity,
        unit_price,
        total_price: unit_price * quantity as u16,
    })
}

pub fn apply_guild_purchase(
    gold: &mut u16,
    stock: &mut u8,
    shop: GuildShop,
    commodity: GuildCommodity,
    quantity: u8,
) -> Result<GuildPurchaseOutcome, GuildPurchaseError> {
    let quote = quote_guild_purchase(shop, commodity, quantity)?;
    let Some(stock_after) = stock.checked_add(quantity) else {
        return Err(GuildPurchaseError::StockCap {
            current: *stock,
            requested: quantity,
            cap: SHOP_COMMODITY_STOCK_CAP,
        });
    };
    if stock_after > SHOP_COMMODITY_STOCK_CAP {
        return Err(GuildPurchaseError::StockCap {
            current: *stock,
            requested: quantity,
            cap: SHOP_COMMODITY_STOCK_CAP,
        });
    }
    if *gold < quote.total_price {
        return Err(GuildPurchaseError::InsufficientGold {
            available: *gold,
            required: quote.total_price,
        });
    }

    let gold_before = *gold;
    let stock_before = *stock;
    *gold -= quote.total_price;
    *stock = stock_after;

    Ok(GuildPurchaseOutcome {
        quote,
        gold_before,
        gold_after: *gold,
        stock_before,
        stock_after,
    })
}

impl PlayState {
    pub fn buy_arms_item(
        &mut self,
        item_id: usize,
        speaker_index: usize,
    ) -> Result<ArmsPurchaseOutcome, ArmsPurchaseError> {
        let Some(speaker_intelligence) = self.speaker_intelligence(speaker_index) else {
            return Err(ArmsPurchaseError::InvalidItem);
        };
        let Some(stock) = self.equipment_stock.get_mut(item_id) else {
            return Err(ArmsPurchaseError::InvalidItem);
        };
        apply_arms_purchase(&mut self.gold, stock, item_id, speaker_intelligence)
    }

    pub fn sell_arms_item(
        &mut self,
        item_id: usize,
        speaker_index: usize,
    ) -> Result<ArmsSaleOutcome, ArmsSaleError> {
        let Some(speaker_intelligence) = self.speaker_intelligence(speaker_index) else {
            return Err(ArmsSaleError::InvalidItem);
        };
        let Some(stock) = self.equipment_stock.get_mut(item_id) else {
            return Err(ArmsSaleError::InvalidItem);
        };
        apply_arms_sale(&mut self.gold, stock, item_id, speaker_intelligence)
    }

    fn speaker_intelligence(&self, speaker_index: usize) -> Option<u8> {
        self.party.get(speaker_index)?;
        if speaker_index == 0 {
            Some(self.avatar_stats.intelligence)
        } else {
            self.party_intelligence.get(speaker_index).copied()
        }
    }

    pub fn buy_guild_commodity(
        &mut self,
        shop: GuildShop,
        commodity: GuildCommodity,
        quantity: u8,
    ) -> Result<GuildPurchaseOutcome, GuildPurchaseError> {
        match commodity {
            GuildCommodity::Keys => {
                apply_guild_purchase(&mut self.gold, &mut self.keys, shop, commodity, quantity)
            }
            GuildCommodity::Gems => {
                apply_guild_purchase(&mut self.gold, &mut self.gems, shop, commodity, quantity)
            }
            GuildCommodity::Torches => {
                apply_guild_purchase(&mut self.gold, &mut self.torches, shop, commodity, quantity)
            }
        }
    }

    pub fn buy_shipwright_vehicle(
        &mut self,
        shipwright: Shipwright,
        kind: ShipwrightPurchaseKind,
        delivery_x: usize,
        delivery_y: usize,
    ) -> Result<ShipwrightPurchaseOutcome, ShipwrightPurchaseError> {
        let Some(return_world) = self.return_world.as_mut() else {
            return Err(ShipwrightPurchaseError::NoReturnWorld);
        };
        apply_shipwright_purchase(
            &mut self.gold,
            &mut return_world.pending_vehicle,
            shipwright,
            kind,
            delivery_x,
            delivery_y,
        )
    }

    pub fn buy_horse(
        &mut self,
        stable: Stable,
        x: usize,
        y: usize,
    ) -> Result<HorsePurchaseOutcome, HorsePurchaseError> {
        let quote = quote_horse_purchase(stable);
        if self.gold < quote.price {
            return Err(HorsePurchaseError::InsufficientGold {
                available: self.gold,
                required: quote.price,
            });
        }
        let Some(z) = self.current_floor() else {
            return Err(HorsePurchaseError::NoCurrentFloor);
        };
        let horse = horse_purchase_active_object(x, y, z);
        let gold_before = self.gold;
        let Some(active_object_slot) = self.allocate_active_object_slot(horse) else {
            return Err(HorsePurchaseError::NoActiveObjectSlot);
        };
        self.gold -= quote.price;

        Ok(HorsePurchaseOutcome {
            quote,
            gold_before,
            gold_after: self.gold,
            active_object_slot,
            horse,
        })
    }

    pub fn buy_healer_treatment(
        &mut self,
        healer: Healer,
        treatment: HealerTreatment,
        target_index: usize,
    ) -> Result<HealerTreatmentOutcome, HealerTreatmentError> {
        if target_index >= self.party.len() {
            return Err(HealerTreatmentError::InvalidTarget {
                party_len: self.party.len(),
                requested: target_index,
            });
        }

        let member = self.party[target_index];
        match treatment {
            HealerTreatment::Cure if member.status != b'P' => {
                return Err(HealerTreatmentError::Untreatable);
            }
            HealerTreatment::Heal if !member.living() || member.hp >= member.max_hp => {
                return Err(HealerTreatmentError::Untreatable);
            }
            HealerTreatment::Resurrect if member.status != b'D' => {
                return Err(HealerTreatmentError::Untreatable);
            }
            _ => {}
        }

        let quote = quote_healer_treatment(healer, treatment);
        if let HealerTreatmentFee::Price(required) = quote.fee {
            if self.gold < required {
                return Err(HealerTreatmentError::InsufficientGold {
                    available: self.gold,
                    required,
                });
            }
        }

        let gold_before = self.gold;
        let status_before = member.status;
        let hp_before = member.hp;

        if let HealerTreatmentFee::Price(required) = quote.fee {
            self.gold -= required;
        }

        match treatment {
            HealerTreatment::Cure => {
                self.party[target_index].status = b'G';
            }
            HealerTreatment::Heal => {
                self.party[target_index].heal_to_max();
            }
            HealerTreatment::Resurrect => {
                let max_hp = self
                    .resurrect_party_member_to_hp(target_index, 1)
                    .expect("target status checked before healer resurrection");
                self.party[target_index].hp = max_hp;
            }
        }

        Ok(HealerTreatmentOutcome {
            quote,
            target_index,
            gold_before,
            gold_after: self.gold,
            status_before,
            status_after: self.party[target_index].status,
            hp_before,
            hp_after: self.party[target_index].hp,
            max_hp_after: self.party[target_index].max_hp,
        })
    }

    pub fn buy_reagent(
        &mut self,
        herbalist: Herbalist,
        reagent: Reagent,
        quantity: u8,
    ) -> Result<ReagentPurchaseOutcome, ReagentPurchaseError> {
        apply_reagent_purchase(
            &mut self.gold,
            &mut self.reagents[reagent.inventory_index()],
            herbalist,
            reagent,
            quantity,
        )
    }

    pub fn buy_tavern_round_drink(
        &mut self,
        tavern: Tavern,
    ) -> Result<TavernDrinkOutcome, TavernDrinkError> {
        let living_party_members = self
            .party
            .iter()
            .filter(|member| member.living())
            .count()
            .min(u8::MAX as usize) as u8;
        apply_tavern_round_drink(&mut self.gold, tavern, living_party_members)
    }

    pub fn buy_blue_boar_drink(
        &mut self,
        choice: BlueBoarDrinkChoice,
    ) -> Result<TavernDrinkOutcome, TavernDrinkError> {
        apply_blue_boar_drink(&mut self.gold, choice)
    }

    pub fn buy_provisions(
        &mut self,
        tavern: Tavern,
        quantity: u16,
    ) -> Result<ProvisionPurchaseOutcome, ProvisionPurchaseError> {
        apply_provision_purchase(&mut self.gold, &mut self.food, tavern, quantity)
    }

    pub fn buy_sage_rumour(
        &mut self,
        topics: &[SageTopic],
        input: &str,
    ) -> Result<SageRumourOutcome, SageRumourError> {
        apply_sage_rumour_purchase(&mut self.gold, topics, input)
    }

    pub fn pay_inn_rest(
        &mut self,
        inn: Inn,
        adjusted_room_rate: u16,
    ) -> Result<InnRestOutcome, InnError> {
        apply_inn_rest_payment(&mut self.gold, inn, self.party.len(), adjusted_room_rate)
    }

    pub fn leave_inn_companion(
        &mut self,
        scene_marker: u8,
        party_index: usize,
        adjusted_lodging_charge: u16,
    ) -> Result<InnLeaveOutcome, InnError> {
        apply_inn_leave_guest(
            &mut self.gold,
            &mut self.inn_registry,
            scene_marker,
            &mut self.party,
            &mut self.party_names,
            &mut self.party_stay_counters,
            &mut self.party_strengths,
            &mut self.party_intelligence,
            &mut self.party_experience,
            &mut self.party_equipment,
            party_index,
            adjusted_lodging_charge,
        )
    }

    pub fn pickup_inn_guest(
        &mut self,
        scene_marker: u8,
        registry_index: usize,
        adjusted_lodging_charge: u16,
    ) -> Result<InnPickupOutcome, InnError> {
        apply_inn_pickup_guest(
            &mut self.gold,
            &mut self.inn_registry,
            scene_marker,
            &mut self.party,
            &mut self.party_names,
            &mut self.party_stay_counters,
            &mut self.party_strengths,
            &mut self.party_intelligence,
            &mut self.party_experience,
            &mut self.party_equipment,
            registry_index,
            adjusted_lodging_charge,
        )
    }
}
