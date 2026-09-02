//! Shop pricing and transaction helpers.

use crate::*;

pub const SHOP_COMMODITY_STOCK_CAP: u8 = 99;

/// `shops.md §6` arms-shop pricing percent denominator. The shop's
/// quote formula divides by 100 to express the Intelligence
/// adjustment as a percentage rather than a raw scalar.
pub const ARMS_SHOP_PERCENT_DENOMINATOR: i32 = 100;
/// `shops.md §6` arms-shop Intelligence weighting factor. The Buy
/// quote subtracts `3 * intelligence` percentage points from 100
/// before scaling the base price; the Sell offer multiplies the
/// base price by the same `3 * intelligence` percentage before the
/// `+1` minimum offer.
pub const ARMS_SHOP_INTELLIGENCE_WEIGHT: i32 = 3;
/// `shops.md §6` arms-shop Sell minimum-offer bias. The Sell offer
/// is `floor(base * 3 * intelligence / 100) + 1`, so every accepted
/// Sell credits at least one gold even when intelligence is zero.
pub const ARMS_SHOP_SELL_MIN_OFFER_BIAS: u32 = 1;
/// `shops.md §§6-8.1` arms-shop Buy menus expose at most eight
/// per-shop stock candidates, rendered as menu letters `a` through
/// `h`. The raw resident sentinel value is a loader concern; runtime
/// helpers operate on the decoded non-sentinel prefix length.
pub const ARMS_SHOP_STOCK_TABLE_LEN: usize = 8;

/// `shops.md §7` / public issue #41 scene-local arms-shop rows. The
/// stock arrays are the published `a..h` resident menu bytes; `0xFF`
/// terminates each row while `0x00` remains a valid equipment id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmsShop {
    IolosBows,
    NaughtyNomaans,
    ArmsOfJustice,
    DarkwatchArmoury,
    ThePaladinsProtectorate,
    NorthStarArmoury,
    BuccaneersBooty,
    TheShatteredShield,
    SiegeCrafters,
}

impl ArmsShop {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::IolosBows => "Iolo's Bows",
            Self::NaughtyNomaans => "Naughty Nomaan's",
            Self::ArmsOfJustice => "Arms of Justice",
            Self::DarkwatchArmoury => "Darkwatch Armoury",
            Self::ThePaladinsProtectorate => "The Paladin's Protectorate!",
            Self::NorthStarArmoury => "North Star Armoury",
            Self::BuccaneersBooty => "Buccaneers Booty",
            Self::TheShatteredShield => "The Shattered Shield",
            Self::SiegeCrafters => "Siege Crafters",
        }
    }

    pub const fn stock_table(self) -> ArmsStockTable {
        match self {
            Self::IolosBows => ArmsStockTable::from_raw([16, 17, 26, 27, 28, 29, 36, 0xFF]),
            Self::NaughtyNomaans => ArmsStockTable::from_raw([19, 24, 46, 22, 3, 6, 25, 0xFF]),
            Self::ArmsOfJustice => ArmsStockTable::from_raw([0, 9, 10, 18, 21, 37, 38, 0xFF]),
            Self::DarkwatchArmoury => ArmsStockTable::from_raw([2, 4, 11, 23, 30, 24, 31, 0xFF]),
            Self::ThePaladinsProtectorate => {
                ArmsStockTable::from_raw([32, 33, 34, 2, 5, 12, 14, 0xFF])
            }
            Self::NorthStarArmoury => ArmsStockTable::from_raw([1, 7, 13, 14, 30, 37, 43, 0xFF]),
            Self::BuccaneersBooty => ArmsStockTable::from_raw([0, 10, 16, 20, 23, 19, 42, 0xFF]),
            Self::TheShatteredShield => ArmsStockTable::from_raw([7, 32, 36, 27, 31, 44, 45, 0xFF]),
            Self::SiegeCrafters => ArmsStockTable::from_raw([1, 13, 28, 29, 34, 22, 25, 0xFF]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArmsStockTable {
    pub item_ids: [u8; ARMS_SHOP_STOCK_TABLE_LEN],
    pub len: u8,
}

impl ArmsStockTable {
    pub const EMPTY: Self = Self {
        item_ids: [0; ARMS_SHOP_STOCK_TABLE_LEN],
        len: 0,
    };

    pub const fn new(item_ids: [u8; ARMS_SHOP_STOCK_TABLE_LEN], len: u8) -> Self {
        let len = if len as usize > ARMS_SHOP_STOCK_TABLE_LEN {
            ARMS_SHOP_STOCK_TABLE_LEN as u8
        } else {
            len
        };
        Self { item_ids, len }
    }

    pub const fn from_raw(raw: [u8; ARMS_SHOP_STOCK_TABLE_LEN]) -> Self {
        let mut item_ids = [0; ARMS_SHOP_STOCK_TABLE_LEN];
        let mut len = 0usize;
        while len < ARMS_SHOP_STOCK_TABLE_LEN {
            if raw[len] == 0xFF {
                break;
            }
            item_ids[len] = raw[len];
            len += 1;
        }
        Self {
            item_ids,
            len: len as u8,
        }
    }

    pub const fn len(self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmsStockLetterError {
    InvalidLetter,
    EmptySlot,
}

/// `shops.md §8.1`: convert an arms-shop Buy menu letter into its
/// zero-based table slot. Visible choices are `a` through `h`;
/// upper-case input is accepted by the same case-insensitive input
/// convention used by the rest of the shop menu classifiers.
pub const fn arms_shop_stock_letter_index(byte: u8) -> Option<usize> {
    let folded = if byte >= b'A' && byte <= b'Z' {
        byte + (b'a' - b'A')
    } else {
        byte
    };
    if folded >= b'a' && folded < b'a' + ARMS_SHOP_STOCK_TABLE_LEN as u8 {
        Some((folded - b'a') as usize)
    } else {
        None
    }
}

/// `shops.md §§6-8.1`: resolve a displayed `a..h` arms-shop Buy
/// choice to the direct equipment item id stored in the current
/// shop's decoded stock table. There is intentionally no item-id
/// translation layer here.
pub const fn arms_shop_stock_item_for_letter(
    table: ArmsStockTable,
    byte: u8,
) -> Result<u8, ArmsStockLetterError> {
    let index = match arms_shop_stock_letter_index(byte) {
        Some(index) => index,
        None => return Err(ArmsStockLetterError::InvalidLetter),
    };
    if index >= table.len() {
        return Err(ArmsStockLetterError::EmptySlot);
    }
    Ok(table.item_ids[index])
}

/// `shops.md §6` arms-shop Buy quote. The shop's quote is the
/// canonical base price plus the integer-truncated Intelligence
/// adjustment `base * (100 - 3 * intelligence) / 100`. The same
/// item therefore quotes differently when a different party member
/// is speaking. Saturating math guards against absurd inputs.
pub const fn arms_shop_buy_quote(base_price: u16, speaker_intelligence: u8) -> u16 {
    let factor: i32 =
        ARMS_SHOP_PERCENT_DENOMINATOR - ARMS_SHOP_INTELLIGENCE_WEIGHT * speaker_intelligence as i32;
    let adjustment = (base_price as i32 * factor) / ARMS_SHOP_PERCENT_DENOMINATOR;
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
    let prod =
        base_price as u32 * ARMS_SHOP_INTELLIGENCE_WEIGHT as u32 * speaker_intelligence as u32;
    let offer = prod / ARMS_SHOP_PERCENT_DENOMINATOR as u32 + ARMS_SHOP_SELL_MIN_OFFER_BIAS;
    if offer > u16::MAX as u32 {
        u16::MAX
    } else {
        offer as u16
    }
}

/// `shops.md §8.1` arms-shop entry-menu outcome. After the
/// randomised "Hail, friend! Wouldst thou Buy or Sell?" greeting,
/// the player picks `B` (Buy), `S` (Sell), or anything else (Exit
/// with the randomised farewell). The shipped flow does not have a
/// re-prompt branch: any non-B/S input including Space exits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmsShopAction {
    /// `B` (case-insensitive) — enter the Buy listing.
    Buy,
    /// `S` (case-insensitive) — enter the Sell browser.
    Sell,
    /// Space or any other key — exit with the randomised farewell.
    Exit,
}

/// `shops.md §8.1`: classify one keystroke for the arms-shop
/// entry menu (weaponsmith / armourer). The caller has already
/// applied the input case fold; this helper accepts both upper-
/// and lower-case `B` / `S` for uppercase-naive callers.
pub const fn arms_shop_action(byte: u8) -> ArmsShopAction {
    match byte {
        b'B' | b'b' => ArmsShopAction::Buy,
        b'S' | b's' => ArmsShopAction::Sell,
        _ => ArmsShopAction::Exit,
    }
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
/// `formats/shoppe-dat.md §2`: 194 of the 196 record slots are
/// non-empty; the remaining 2 are empty-trailer padding.
/// Anchored to SHOPPE_DAT_RECORD_SLOTS - 2 so the non-empty
/// record count tracks the slot count.
pub const SHOPPE_DAT_NONEMPTY_RECORDS: usize = SHOPPE_DAT_RECORD_SLOTS - 2;

/// `formats/shoppe-dat.md §6` per-cluster `SHOPPE.DAT` record-id
/// ranges. Bands that the spec publishes as one contiguous run get a
/// `_FIRST`/`_LAST` pair — that pair *is* the assertion that the run
/// has no holes. Bands the spec publishes as interleaved or holed
/// (sage, healer) get an explicit `_BANDS` list instead, because a
/// `_FIRST`/`_LAST` pair would silently claim records that belong to
/// another cluster.
pub const SHOPPE_RECORDS_SHARED_BARKS_FIRST: usize = 0;
pub const SHOPPE_RECORDS_SHARED_BARKS_LAST: usize = 7;

/// `shops.md §4.1` substitution placeholder bytes the bark renderer
/// expands inline from runtime shop state. None of the shipped record
/// text uses these as literal punctuation, so the renderer always
/// expands them rather than rendering the byte as itself.
pub const SHOP_PLACEHOLDER_GOLD: u8 = b'%';
pub const SHOP_PLACEHOLDER_QUANTITY: u8 = b'^';
pub const SHOP_PLACEHOLDER_VENDOR_NAME: u8 = b'$';
pub const SHOP_PLACEHOLDER_ITEM_NAME: u8 = b'&';
pub const SHOP_PLACEHOLDER_PLACE_NAME: u8 = b'*';
pub const SHOP_PLACEHOLDER_SHOP_NAME: u8 = b'#';
pub const SHOP_PLACEHOLDER_TIME_OF_DAY: u8 = b'@';

/// `shops.md §4.1` placeholder kind classifier for the bark renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopPlaceholderKind {
    /// `%` — gold amount; decimal digits with no thousands separator.
    Gold,
    /// `^` — quantity (bottles, ounces, hours, etc.).
    Quantity,
    /// `$` — vendor / shopkeeper display name.
    VendorName,
    /// `&` — item name (currently quoted item).
    ItemName,
    /// `*` — place name (town, landmark; sage rumour).
    PlaceName,
    /// `#` — shop name.
    ShopName,
    /// `@` — time-of-day word (morning/afternoon/evening, read fresh
    /// from the world clock on every render).
    TimeOfDay,
}

/// `shops.md §4.1`: classify a byte as one of the seven substitution
/// placeholders, or `None` for any other byte (the renderer emits
/// those literally or routes high-bit bytes through the phrase-token
/// dictionary instead).
pub const fn shop_placeholder_kind(byte: u8) -> Option<ShopPlaceholderKind> {
    Some(match byte {
        SHOP_PLACEHOLDER_GOLD => ShopPlaceholderKind::Gold,
        SHOP_PLACEHOLDER_QUANTITY => ShopPlaceholderKind::Quantity,
        SHOP_PLACEHOLDER_VENDOR_NAME => ShopPlaceholderKind::VendorName,
        SHOP_PLACEHOLDER_ITEM_NAME => ShopPlaceholderKind::ItemName,
        SHOP_PLACEHOLDER_PLACE_NAME => ShopPlaceholderKind::PlaceName,
        SHOP_PLACEHOLDER_SHOP_NAME => ShopPlaceholderKind::ShopName,
        SHOP_PLACEHOLDER_TIME_OF_DAY => ShopPlaceholderKind::TimeOfDay,
        _ => return None,
    })
}

/// `formats/shoppe-dat.md §6`: shoppe-record bands. The shared-
/// barks, arms-descriptions, arms-sell, and tavern bands tile
/// contiguously from record 0 upward. Anchor each *_FIRST to
/// the previous band's *_LAST + 1 so resizing any band shifts
/// the later bands automatically.
pub const SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST: usize = SHOPPE_RECORDS_SHARED_BARKS_LAST + 1;
pub const SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST: usize = 48;

pub const SHOPPE_RECORDS_ARMS_SELL_FIRST: usize = SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST + 1;
pub const SHOPPE_RECORDS_ARMS_SELL_LAST: usize = 56;

/// `formats/shoppe-dat.md §6`: "Tavern, meal-counter, and related
/// interactive prompts and menus" span records **57-91** as one run,
/// including the state list records `69-72`, the state follow-ups
/// `73-76`, the provision quotes `77-82`, and the table-scraps
/// outcome `90`. The sage records interleaved inside it (below) do
/// not shorten the tavern band.
pub const SHOPPE_RECORDS_TAVERN_FIRST: usize = SHOPPE_RECORDS_ARMS_SELL_LAST + 1;
pub const SHOPPE_RECORDS_TAVERN_LAST: usize = 91;

/// `formats/shoppe-dat.md §6`: the sage rumour records are
/// **interleaved inside** the tavern band and are **not contiguous**:
/// `84` fee quote, `85-88` success templates, `91` paying-customers
/// refusal. Records `89` and `90` sitting between them belong to
/// tavern branches, not to the sage — so a `_FIRST`/`_LAST` pair
/// over `84..=91` would claim two records this cluster does not own.
/// The published sub-runs are listed explicitly instead.
pub const SHOPPE_RECORDS_SAGE_BANDS: [(usize, usize); 2] = [
    (
        SAGE_RUMOUR_FEE_QUOTE_RECORD,
        SAGE_RUMOUR_SUCCESS_RECORD_LAST,
    ),
    (
        SAGE_RUMOUR_SHORT_FUNDS_RECORD,
        SAGE_RUMOUR_SHORT_FUNDS_RECORD,
    ),
];
/// `formats/shoppe-dat.md §6`: the two records that fall between the
/// sage's sub-runs and belong to tavern branches instead.
pub const SHOPPE_RECORDS_SAGE_INTERLEAVED_TAVERN: [usize; 2] = [89, 90];

pub const SHOPPE_RECORDS_HORSE_TRADER_FIRST: usize = SHOPPE_RECORDS_TAVERN_LAST + 1;
pub const SHOPPE_RECORDS_HORSE_TRADER_LAST: usize = 104;

pub const SHOPPE_RECORDS_SHIP_BROKER_FIRST: usize = SHOPPE_RECORDS_HORSE_TRADER_LAST + 1;
pub const SHOPPE_RECORDS_SHIP_BROKER_LAST: usize = 126;

pub const SHOPPE_RECORDS_REAGENT_FIRST: usize = SHOPPE_RECORDS_SHIP_BROKER_LAST + 1;
pub const SHOPPE_RECORDS_REAGENT_LAST: usize = 146;

pub const SHOPPE_RECORDS_GUILD_FIRST: usize = 148;
pub const SHOPPE_RECORDS_GUILD_LAST: usize = 162;

/// `formats/shoppe-dat.md §6`: the healer/sanctum cluster is
/// published as "163 and 165-173" — record `164` is **not** part of
/// it, so a `_FIRST`/`_LAST` pair over `163..=173` would assert a
/// contiguity the file does not have. The two sub-runs are listed
/// explicitly instead.
pub const SHOPPE_RECORDS_HEALER_BANDS: [(usize, usize); 2] = [
    (SHOPPE_RECORDS_GUILD_LAST + 1, SHOPPE_RECORDS_GUILD_LAST + 1),
    (165, 173),
];
/// `formats/shoppe-dat.md §6`: the record between the healer
/// cluster's two sub-runs, which the published table does not assign
/// to the healer.
pub const SHOPPE_RECORDS_HEALER_EXCLUDED: usize = 164;

/// `formats/shoppe-dat.md §6`: the innkeeper band starts one past the
/// end of the healer cluster's last sub-run.
pub const SHOPPE_RECORDS_INNKEEPER_FIRST: usize = SHOPPE_RECORDS_HEALER_BANDS[1].1 + 1;
/// `formats/shoppe-dat.md §6`: the innkeeper band ends at the
/// last non-empty record. Anchored to
/// SHOPPE_DAT_NONEMPTY_RECORDS - 1 so the last band end
/// derives from the non-empty record count.
pub const SHOPPE_RECORDS_INNKEEPER_LAST: usize = SHOPPE_DAT_NONEMPTY_RECORDS - 1;

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
/// `shops.md §6` (provisions) caps purchased food at the
/// engine-wide party food counter cap. Anchored to
/// [`crate::PARTY_FOOD_CAP`] so the shop clamp and the carrier
/// cap stay one value.
pub const SHOP_FOOD_STOCK_CAP: u16 = crate::PARTY_FOOD_CAP;
/// `shops.md §6.1`: one tavern provision unit is a pack of twenty-five
/// servings, not one point on the shared food counter.
pub const TAVERN_PROVISION_PACK_SERVINGS: u16 = 25;
/// `shops.md §8.5`: when no provision unit can be afforded, a party below
/// three food receives one charitable serving; parties at three or more take
/// the ordinary no-need refusal instead.
pub const TAVERN_PROVISION_CHARITY_THRESHOLD: u16 = 3;
/// `shops.md §6` clamps the gold counter after every shop
/// outcome (sale credit, surcharge, refund) at the same word-sized
/// 9999 cap inventory.md §2 documents for the party gold counter.
/// Anchored to [`crate::PARTY_GOLD_CAP`] for a single source.
pub const SHOP_GOLD_CAP: u16 = crate::PARTY_GOLD_CAP;

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

/// `shops.md §8.2` guildmaster (magic shop) entry-menu outcome.
/// After the greeting, the player picks `a` Keys, `b` Gems, `c`
/// Torches, or any other key (Exit). The shipped flow does not
/// re-prompt: any non-`a`/`b`/`c` input including Space exits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildShopAction {
    /// `a` (case-insensitive) — Keys.
    Purchase(GuildCommodity),
    /// Any other key — exit the menu.
    Exit,
}

/// `shops.md §8.2`: classify one keystroke for the guildmaster
/// entry menu. Caller has applied the input case fold; this helper
/// also accepts the upper-case `A` / `B` / `C` variants so
/// uppercase-naive callers pass through cleanly.
pub const fn guild_shop_action(byte: u8) -> GuildShopAction {
    match byte {
        b'a' | b'A' => GuildShopAction::Purchase(GuildCommodity::Keys),
        b'b' | b'B' => GuildShopAction::Purchase(GuildCommodity::Gems),
        b'c' | b'C' => GuildShopAction::Purchase(GuildCommodity::Torches),
        _ => GuildShopAction::Exit,
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

/// `shops.md §8.7` shipwright entry menu outcome. The Talk-triggered
/// vehicle-sale flow opens with a small letter menu: `F` offers
/// Frigates, `S` offers Skiffs, while Space or Escape exits. Any
/// other key silently re-prompts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipwrightMenuAction {
    /// `F` (case-insensitive) — Frigate purchase quote.
    Purchase(ShipwrightPurchaseKind),
    /// Space or Escape — exit the menu.
    Exit,
    /// Any other byte — silently re-prompt.
    Discard,
}

/// `shops.md §8.7`: classify one keystroke for the shipwright entry
/// menu. The caller has already applied the input-layer case fold;
/// this helper also accepts the lower-case `f` / `s` variants for
/// uppercase-naive callers.
pub const fn shipwright_menu_action(byte: u8) -> ShipwrightMenuAction {
    match byte {
        b'F' | b'f' => ShipwrightMenuAction::Purchase(ShipwrightPurchaseKind::Frigate),
        b'S' | b's' => ShipwrightMenuAction::Purchase(ShipwrightPurchaseKind::Skiff),
        b' ' | 0x1B => ShipwrightMenuAction::Exit,
        _ => ShipwrightMenuAction::Discard,
    }
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

/// `shops.md §8.3` healer service-menu outcome. After the entry
/// `Y`/`N` prompt accepts, the healer's service menu polls one
/// keystroke: `C` Cure, `H` Heal, `R` Resurrect, Space or Return
/// exits, anything else re-prompts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealerServiceAction {
    /// `C` (case-insensitive) — Cure: remove Poisoned status.
    Treatment(HealerTreatment),
    /// Space or Return — leave the service menu and print the
    /// exit line.
    Exit,
    /// Any other byte — silently re-prompt.
    Discard,
}

/// `shops.md §8.3`: classify one keystroke for the healer service
/// menu. The caller has already applied the input case fold; this
/// helper also accepts the lower-case `c` / `h` / `r` variants for
/// uppercase-naive callers.
pub const fn healer_service_action(byte: u8) -> HealerServiceAction {
    match byte {
        b'C' | b'c' => HealerServiceAction::Treatment(HealerTreatment::Cure),
        b'H' | b'h' => HealerServiceAction::Treatment(HealerTreatment::Heal),
        b'R' | b'r' => HealerServiceAction::Treatment(HealerTreatment::Resurrect),
        b' ' | b'\r' | b'\n' => HealerServiceAction::Exit,
        _ => HealerServiceAction::Discard,
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
        HealerTreatment::Cure => matches!(status, CharacterStatus::Poisoned),
        HealerTreatment::Heal => !matches!(status, CharacterStatus::Dead) && hp < max_hp,
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

    /// `catalogs/spell-list.md §3`: one-byte recipe-mask bit. The
    /// M-Mix command compares the player's selected reagent set
    /// against the spell's recipe by ORing these bit values. Bit
    /// order matches the published table: Sulfur Ash is the high
    /// bit (`0x80`); Mandrake is the low bit (`0x01`).
    pub const fn recipe_bit(self) -> u8 {
        match self {
            Self::SulfurAsh => 0x80,
            Self::Ginseng => 0x40,
            Self::Garlic => 0x20,
            Self::SpiderSilk => 0x10,
            Self::BloodMoss => 0x08,
            Self::BlackPearl => 0x04,
            Self::Nightshade => 0x02,
            Self::Mandrake => 0x01,
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

/// `shops.md §8.5` tavern entry-prompt outcome. The shopkeeper
/// greets the Avatar and asks whether they want a drink: `Y`
/// enters the local menu, `N` or Space leaves, any other key
/// silently re-prompts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TavernDrinkPrompt {
    /// `Y` (case-insensitive) — enter the tavern's drink menu.
    Enter,
    /// `N` (case-insensitive) or Space — leave with the farewell.
    Leave,
    /// Any other byte — silently re-prompt.
    Discard,
}

/// `shops.md §8.5`: classify one keystroke for the tavern entry
/// drink prompt. Lower-case `y` / `n` accepted for uppercase-naive
/// callers.
pub const fn tavern_drink_prompt(byte: u8) -> TavernDrinkPrompt {
    match byte {
        b'Y' | b'y' => TavernDrinkPrompt::Enter,
        b'N' | b'n' | b' ' => TavernDrinkPrompt::Leave,
        _ => TavernDrinkPrompt::Discard,
    }
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
    pub completion: ProvisionPurchaseCompletion,
}

/// Why the provision loop returned after accepting a nonzero quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvisionPurchaseCompletion {
    /// The requested quantity finished, or food reached the 9999 ceiling.
    /// Both paths use the completed-purchase exit and may take the surcharge.
    Completed,
    /// At least one unit was served before gold ran out. The purchased units
    /// remain, but the post-transaction surcharge is skipped.
    GoldExhausted,
    /// No unit was affordable and food was below three, so the shopkeeper
    /// supplied one free serving and ended the visit.
    Charity,
}

impl ProvisionPurchaseCompletion {
    pub const fn surcharge_applies(self) -> bool {
        matches!(self, Self::Completed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvisionPurchaseError {
    ZeroQuantity,
    NoNeed,
}

/// `shops.md §8.4`: the inn registry is "a 16-slot, save-backed
/// resident view... a shifted legacy view over the save image
/// rather than an independent post-roster block" — so the registry
/// cap is the same 16 roster slots the save file already exposes.
/// Anchored to [`crate::SAVE_ROSTER_SLOT_COUNT`] to keep the shop
/// path and the save image in lockstep.
pub const INN_REGISTRY_CAP: usize = crate::SAVE_ROSTER_SLOT_COUNT;
/// `shops.md §8.4` Active party-size gate for the inn `L` Leave
/// flow: leave is rejected when the travelling party is already
/// at the engine-wide party cap. Anchored to
/// [`crate::SAVE_PARTY_SIZE_MAX`] so the inn gate and the save
/// file's roster cap stay a single value.
pub const INN_PARTY_CAP: usize = crate::SAVE_PARTY_SIZE_MAX as usize;
/// `shops.md §8.4`: maximum value the inn's guest-stay counter
/// can reach. The same counter is the character record's
/// month-counter byte that the 28-day rollover ages each month,
/// so this cap is the same value as
/// [`crate::CHARACTER_MONTH_COUNTER_CAP`]. Anchored through to
/// the month-counter cap so the inn's stay-counter ceiling and
/// the character record's month-counter ceiling stay in lockstep.
pub const INN_STAY_COUNTER_CAP: u8 = crate::CHARACTER_MONTH_COUNTER_CAP;

/// `shops.md §8.4` Leave-companion deposit unit count. The deposit
/// debited when the player leaves a companion at an inn is the local
/// base room rate multiplied by this many units.
pub const INN_LEAVE_DEPOSIT_ROOM_RATE_UNITS: u8 = 10;

/// `shops.md §8.4`: Leave-companion deposit calculated from the
/// inn's base room rate. Returns the gold amount to debit before the
/// registry transfer completes.
pub const fn inn_leave_companion_deposit(base_room_rate: u16) -> u16 {
    base_room_rate * INN_LEAVE_DEPOSIT_ROOM_RATE_UNITS as u16
}

pub const fn shop_intelligence_adjusted_price(raw: u16, speaker_intelligence: u8) -> u16 {
    let factor: i32 =
        ARMS_SHOP_PERCENT_DENOMINATOR - ARMS_SHOP_INTELLIGENCE_WEIGHT * speaker_intelligence as i32;
    let adjustment = (raw as i32 * factor) / ARMS_SHOP_PERCENT_DENOMINATOR;
    let adjusted = raw as i32 + adjustment;
    if adjusted < 0 {
        0
    } else if adjusted > u16::MAX as i32 {
        u16::MAX
    } else {
        adjusted as u16
    }
}

/// `shops.md §8.4`: Pickup bill calculated from the base local
/// lodging charge and the guest's stored stay counter, treating zero
/// as one billable unit (so a same-day pickup still costs one
/// lodging charge).
pub const fn inn_pickup_bill(base_lodging_charge: u16, stay_counter: u8) -> u16 {
    let units = if stay_counter == 0 { 1 } else { stay_counter };
    base_lodging_charge * units as u16
}

pub const fn inn_leave_companion_deposit_for_speaker(inn: Inn, speaker_intelligence: u8) -> u16 {
    shop_intelligence_adjusted_price(
        inn_leave_companion_deposit(inn_base_room_rate(inn)),
        speaker_intelligence,
    )
}

pub const fn inn_pickup_bill_for_speaker(
    inn: Inn,
    stay_counter: u8,
    speaker_intelligence: u8,
) -> u16 {
    inn_pickup_bill(
        shop_intelligence_adjusted_price(
            inn_leave_companion_deposit(inn_base_room_rate(inn)),
            speaker_intelligence,
        ),
        stay_counter,
    )
}

/// `shops.md §8.4` morbid pickup conversion. A guest left at the
/// inn while Poisoned is converted to Dead on pickup: the returned
/// record's status flips to `'D'`, current HP is cleared, and the
/// inn prints "Thy friend has died, by the way." Other stored
/// statuses pass through unchanged.
pub const fn inn_pickup_status_converts_to_dead(stored_status: CharacterStatus) -> bool {
    matches!(stored_status, CharacterStatus::Poisoned)
}

/// `shops.md §8.4` 28-day month-rollover stay-counter cap. Each
/// month rollover bumps the inn registry's per-guest stay counter
/// by one until this cap is reached; the pickup bill multiplies the
/// adjusted lodging charge by the stored counter. Anchored to
/// [`INN_STAY_COUNTER_CAP`] so the persistence cap and the
/// month-rollover cap stay a single value.
pub const INN_STAY_COUNTER_MAX: u8 = INN_STAY_COUNTER_CAP;

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

    /// `shops.md §8.4` per-inn base room rate. Public issue #15
    /// corrected the inn flow to apply the shared speaking-member
    /// Intelligence adjustment after computing each raw bill.
    pub const fn base_room_rate(self) -> u16 {
        match self {
            Self::TheWayfarerInn => 2,
            Self::TheWarriorsStead => 3,
            Self::TheHauntingInn => 2,
            Self::HotelBrittany => 3,
            Self::TheSmugglersInn => 2,
            Self::TheKingsRansomInn => 3,
        }
    }

    /// `shops.md §8.4` per-inn minimum-gold gate. The inn refuses to
    /// open the main menu when party gold is below this floor.
    pub const fn minimum_gold_gate(self) -> u16 {
        match self {
            Self::TheWayfarerInn => 3,
            Self::TheWarriorsStead => 4,
            Self::TheHauntingInn => 3,
            Self::HotelBrittany => 2,
            Self::TheSmugglersInn => 2,
            Self::TheKingsRansomInn => 2,
        }
    }
}

/// `shops.md §8.4` inn main-menu outcome. After the inn-entry guest
/// scan, the main menu accepts three actions: `R` Rest for the
/// night, `L` Leave a companion, `P` Pick up a companion. Other
/// keys silently re-prompt; Space/Escape exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InnMainAction {
    /// `R` (case-insensitive) — Rest for the night.
    Rest,
    /// `L` (case-insensitive) — Leave a companion at the inn.
    LeaveCompanion,
    /// `P` (case-insensitive) — Pick up a previously-left companion.
    PickUpCompanion,
    /// Space or Escape — exit the menu.
    Exit,
    /// Any other byte — silently re-prompt.
    Discard,
}

/// `shops.md §8.4`: classify one keystroke for the inn main menu.
/// Lower-case `r`/`l`/`p` accepted for uppercase-naive callers.
pub const fn inn_main_action(byte: u8) -> InnMainAction {
    match byte {
        b'R' | b'r' => InnMainAction::Rest,
        b'L' | b'l' => InnMainAction::LeaveCompanion,
        b'P' | b'p' => InnMainAction::PickUpCompanion,
        b' ' | 0x1B => InnMainAction::Exit,
        _ => InnMainAction::Discard,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InnGuestRecord {
    /// Index of this guest's slot in the sixteen-slot shifted registry view of
    /// `formats/saved-gam.md` §3.3. The registry overlaps the character roster,
    /// so a guest must be written back at the slot it came from; repacking the
    /// list from slot zero shifts whole roster records.
    pub registry_slot: u8,
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
    pub base_room_rate: u16,
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

/// `shops.md §8.8` sage free-text input character cap.
pub const SAGE_TOPIC_INPUT_LIMIT: usize = 15;

/// `shops.md §2` horse-trader Talk dialog id. Ordinary shops refuse
/// to open their menu when the party is mounted on a horse; only
/// the horse-trader vehicle-sale arm (`0x83`) remains available.
pub const HORSE_TRADER_DIALOG_ID: u8 = 0x83;

/// `shops.md §2` mounted-horse shop-entry gate. Returns `true` when
/// the shop arm should refuse to open because the party is mounted
/// on a horse and the target shop is not the horse trader. The horse
/// trader is the only shop dialog id allowed while mounted.
pub const fn shop_refuses_mounted_horse(dialog_id: u8, mounted_on_horse: bool) -> bool {
    mounted_on_horse && dialog_id != HORSE_TRADER_DIALOG_ID
}

/// `shops.md §8.3` unmatched resident healer cost row. The resident
/// healer cost table has eight rows; the seven shipped healers map
/// to rows 0..=6, but row 7 carries the published `(Cure 1,
/// Heal 70, Resurrect 270)` fees with no shipped healer scene
/// reaching it. Compatible implementations should preserve the
/// values so a custom or modded scene that points at row 7 still
/// gets the same fees the resident engine would have charged.
pub const HEALER_UNMATCHED_ROW_CURE_FEE: u16 = 1;
pub const HEALER_UNMATCHED_ROW_HEAL_FEE: u16 = 70;
pub const HEALER_UNMATCHED_ROW_RESURRECT_FEE: u16 = 270;

/// `shops.md §6.1` published shop affordability refusal barks. The
/// shop kind picks which line to print when the affordability gate
/// rejects a purchase; the bark is presentation only and does not
/// change gold or inventory.
pub const TAVERN_AFFORDABILITY_REFUSAL_BARK: &str = "Beat it!";
pub const UPMARKET_INN_AFFORDABILITY_REFUSAL_BARK: &str = "Highwaymen!";
/// `shops.md §6.1` vehicle-broker partial-afford prefix. Vehicle
/// brokers (horse trader, shipwright) print
/// `"Thou canst afford only "` followed by the affordable count;
/// callers compose the trailing quantity at format time.
pub const VEHICLE_BROKER_PARTIAL_AFFORD_PREFIX: &str = "Thou canst afford only ";

/// Public issue `cleak/u5-spec#13`: the corrected sage flow uses one
/// global resident topic table, not per-sage synthetic rows.
pub const SAGE_RUMOUR_TOPIC_COUNT: usize = 26;

/// Public issue `cleak/u5-spec#13`: paid sage success barks are drawn
/// from the sequential SHOPPE.DAT records 85..=88.
pub const SAGE_RUMOUR_FEE_QUOTE_RECORD: usize = 84;
pub const SAGE_RUMOUR_SUCCESS_RECORD_FIRST: usize = 85;
pub const SAGE_RUMOUR_SUCCESS_RECORD_LAST: usize = 88;
pub const SAGE_RUMOUR_SHORT_FUNDS_RECORD: usize = 91;

/// Public issue `cleak/u5-spec#13` sage rumour-keyword input cap.
/// The input pipeline accepts at most fifteen characters for the
/// sage's free-text keyword. Runtime helpers therefore match only
/// the first fifteen characters instead of treating longer test
/// harness strings as a separate paid-shop error path.
pub const SAGE_KEYWORD_INPUT_LIMIT: usize = 15;

/// `formats/shoppe-dat.md §6`: membership test over a published,
/// possibly non-contiguous record cluster. Bands are inclusive
/// `(first, last)` pairs.
pub const fn shoppe_record_in_bands(record_id: usize, bands: &[(usize, usize)]) -> bool {
    let mut index = 0;
    while index < bands.len() {
        let (first, last) = bands[index];
        if record_id >= first && record_id <= last {
            return true;
        }
        index += 1;
    }
    false
}

/// `formats/shoppe-dat.md §6`: the sage owns `84-88` and `91` only.
/// Records `89` and `90` are tavern branches and are rejected here.
pub const fn sage_rumour_record_id_accepted(record_id: usize) -> bool {
    shoppe_record_in_bands(record_id, &SHOPPE_RECORDS_SAGE_BANDS)
}

/// `shops.md §8.8`: returns `true` when `typed` matches `topic` per
/// the sage's strict topic-boundary rule: case-insensitive prefix
/// equality with the next character either end-of-input or a space.
/// Empty topic strings are rejected; partial prefixes never match
/// longer topics, and longer words that merely start with a topic
/// are rejected.
pub fn sage_topic_matches(typed: &str, topic: &str) -> bool {
    if topic.is_empty() {
        return false;
    }
    let typed_bytes = typed.as_bytes();
    let topic_bytes = topic.as_bytes();
    if typed_bytes.len() < topic_bytes.len() {
        return false;
    }
    let mut i = 0;
    while i < topic_bytes.len() {
        if !typed_bytes[i].eq_ignore_ascii_case(&topic_bytes[i]) {
            return false;
        }
        i += 1;
    }
    if typed_bytes.len() == topic_bytes.len() {
        return true;
    }
    typed_bytes[topic_bytes.len()] == b' '
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SageRumourEntry {
    pub keyword: &'static str,
    pub subject: &'static str,
    pub destination: &'static str,
    pub fee: u16,
}

pub type SageRumourTable = [SageRumourEntry; SAGE_RUMOUR_TOPIC_COUNT];

pub const SAGE_RUMOUR_TABLE: SageRumourTable = [
    SageRumourEntry {
        keyword: "hone",
        subject: "Malik",
        destination: "Moonglow",
        fee: 50,
    },
    SageRumourEntry {
        keyword: "comp",
        subject: "Greyson",
        destination: "Britain",
        fee: 75,
    },
    SageRumourEntry {
        keyword: "valo",
        subject: "Trian",
        destination: "Jhelom",
        fee: 50,
    },
    SageRumourEntry {
        keyword: "just",
        subject: "Jeremy",
        destination: "Yew",
        fee: 50,
    },
    SageRumourEntry {
        keyword: "sacr",
        subject: "Rew",
        destination: "Minoc",
        fee: 75,
    },
    SageRumourEntry {
        keyword: "hono",
        subject: "Gruman",
        destination: "Trinsic",
        fee: 75,
    },
    SageRumourEntry {
        keyword: "spir",
        subject: "Saul",
        destination: "Skara Brae",
        fee: 25,
    },
    SageRumourEntry {
        keyword: "humi",
        subject: "Shirita",
        destination: "New Magincia",
        fee: 50,
    },
    SageRumourEntry {
        keyword: "dece",
        subject: "Malifora",
        destination: "Moonglow",
        fee: 100,
    },
    SageRumourEntry {
        keyword: "desp",
        subject: "Annon",
        destination: "Britain",
        fee: 150,
    },
    SageRumourEntry {
        keyword: "dest",
        subject: "Trian",
        destination: "Jhelom",
        fee: 75,
    },
    SageRumourEntry {
        keyword: "wron",
        subject: "Felespar",
        destination: "Yew",
        fee: 150,
    },
    SageRumourEntry {
        keyword: "cove",
        subject: "the mother of Rew",
        destination: "Minoc",
        fee: 75,
    },
    SageRumourEntry {
        keyword: "sham",
        subject: "Sindar",
        destination: "Trinsic",
        fee: 100,
    },
    SageRumourEntry {
        keyword: "hyth",
        subject: "Kaiko",
        destination: "New Magincia",
        fee: 100,
    },
    SageRumourEntry {
        keyword: "crow",
        subject: "Terrance",
        destination: "Britain",
        fee: 200,
    },
    SageRumourEntry {
        keyword: "scep",
        subject: "Greymarch",
        destination: "Yew",
        fee: 200,
    },
    SageRumourEntry {
        keyword: "amul",
        subject: "Simon and Tessa",
        destination: "a hidden mountain keep",
        fee: 200,
    },
    SageRumourEntry {
        keyword: "fals",
        subject: "Shalineth",
        destination: "the Lycaeum",
        fee: 250,
    },
    SageRumourEntry {
        keyword: "hatr",
        subject: "a daemon",
        destination: "the desert",
        fee: 250,
    },
    SageRumourEntry {
        keyword: "cowa",
        subject: "Lord Malone",
        destination: "Serpent's Hold",
        fee: 250,
    },
    SageRumourEntry {
        keyword: "astr",
        subject: "Zachariah",
        destination: "Moonglow",
        fee: 100,
    },
    SageRumourEntry {
        keyword: "oppr",
        subject: "Tactus",
        destination: "Minoc",
        fee: 50,
    },
    SageRumourEntry {
        keyword: "brit",
        subject: "a daemon",
        destination: "the desert",
        fee: 50,
    },
    SageRumourEntry {
        keyword: "resi",
        subject: "Terrance",
        destination: "Britain",
        fee: 200,
    },
    SageRumourEntry {
        keyword: "unde",
        subject: "Jotham",
        destination: "a lighthouse south of Britain",
        fee: 100,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SageRumourQuote {
    pub entry: SageRumourEntry,
    pub input_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SageRumourOutcome {
    pub quote: SageRumourQuote,
    pub record_id: usize,
    pub rendered: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SageRumourError {
    EmptyInput,
    InputTooLong { limit: usize, actual: usize },
    NoTopicMatch,
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

/// `shops.md §6.2` shop surcharge minimum gold subtracted on a hit.
/// The roll is masked to the low six bits and biased by this value so
/// the surcharge band is `1..=SHOP_SURCHARGE_GOLD_MAX`.
pub const SHOP_SURCHARGE_GOLD_MIN: u16 = 1;
/// `shops.md §6.2` shop surcharge maximum gold subtracted on a hit.
/// The surcharge formula is `(roll & MASK) + MIN`, so the maximum
/// value equals `MASK + MIN` by construction. Anchored to
/// `SHOP_SURCHARGE_ROLL_MASK + SHOP_SURCHARGE_GOLD_MIN` so the
/// surcharge band's upper bound derives from the mask and bias.
pub const SHOP_SURCHARGE_GOLD_MAX: u16 = SHOP_SURCHARGE_ROLL_MASK as u16 + SHOP_SURCHARGE_GOLD_MIN;
/// `shops.md §6.2` mask applied to the surcharge roll seed before
/// adding [`SHOP_SURCHARGE_GOLD_MIN`]. The low six bits give
/// `0..=63`, which biases to a uniform `1..=64`-gold band.
pub const SHOP_SURCHARGE_ROLL_MASK: u8 = 0x3F;
/// `shops.md §6.2` shared town/conversation sentinel value that
/// enables the surcharge. Slot value `0` runs the extra charge;
/// slot values `1`/`2` and the no-slot marker suppress it.
pub const SHOP_SURCHARGE_SENTINEL_ENABLES: u8 = 0;

pub const fn shop_surcharge_from_roll_seed(roll_seed: u8) -> u16 {
    SHOP_SURCHARGE_GOLD_MIN + (roll_seed & SHOP_SURCHARGE_ROLL_MASK) as u16
}

pub fn apply_shop_surcharge(
    gold: &mut u16,
    shared_town_conversation_sentinel: u8,
    roll_seed: u8,
) -> ShopSurchargeOutcome {
    let surcharge = shop_surcharge_from_roll_seed(roll_seed);
    let gold_before = *gold;
    let applied = shared_town_conversation_sentinel == SHOP_SURCHARGE_SENTINEL_ENABLES;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TavernMenuLetters {
    pub round: char,
    pub secondary: char,
    pub provisions: Option<char>,
    pub lore: char,
}

pub const fn tavern_menu_letters(tavern: Tavern) -> TavernMenuLetters {
    match tavern {
        Tavern::TheHonestMeal
        | Tavern::TheWayfarerTavern
        | Tavern::TheSwordAndKeg
        | Tavern::TheCatsLair
        | Tavern::TheFolleyTap => TavernMenuLetters {
            round: 'M',
            secondary: 'A',
            provisions: Some('R'),
            lore: 'C',
        },
        Tavern::TheSlaughteredLamb | Tavern::TheFallenVirgin => TavernMenuLetters {
            round: 'B',
            secondary: 'R',
            provisions: None,
            lore: 'H',
        },
        Tavern::TheHumblePalate => TavernMenuLetters {
            round: 'F',
            secondary: 'S',
            provisions: Some('P'),
            lore: 'A',
        },
        Tavern::TheBlueBoarTavern => TavernMenuLetters {
            round: 'C',
            secondary: 'W',
            provisions: None,
            lore: 'T',
        },
    }
}

/// `shops.md §8.5`: the four tavern menu states select `SHOPPE.DAT`
/// list records 69..=72 and follow-up records 73..=76.
pub const fn tavern_menu_state_index(tavern: Tavern) -> usize {
    match tavern {
        Tavern::TheHonestMeal
        | Tavern::TheWayfarerTavern
        | Tavern::TheSwordAndKeg
        | Tavern::TheCatsLair
        | Tavern::TheFolleyTap => 0,
        Tavern::TheSlaughteredLamb | Tavern::TheFallenVirgin => 1,
        Tavern::TheHumblePalate => 2,
        Tavern::TheBlueBoarTavern => 3,
    }
}

pub const fn tavern_menu_record_id(tavern: Tavern) -> usize {
    69 + tavern_menu_state_index(tavern)
}

pub const fn tavern_follow_up_record_id(tavern: Tavern) -> usize {
    73 + tavern_menu_state_index(tavern)
}

pub const TAVERN_PROVISION_QUOTE_RECORD_FIRST: usize = 77;
pub const TAVERN_PROVISION_QUOTE_RECORD_LAST: usize = 82;
pub const TAVERN_NO_SALE_RECORD_FIRST: usize = 61;
pub const TAVERN_NO_SALE_RECORD_LAST: usize = 64;
pub const TAVERN_TABLE_SCRAPS_RECORD_ID: usize = 90;
pub const TAVERN_BARE_TABLE_SETTING_TILE: u8 = 0x95;
pub const TAVERN_NORTH_FOOD_SETTING_TILE: u8 = 0x9B;
pub const TAVERN_SOUTH_FOOD_SETTING_TILE: u8 = 0x9A;

pub const fn tavern_round_drink_menu_letter(tavern: Tavern) -> char {
    tavern_menu_letters(tavern).round
}

pub const fn tavern_secondary_menu_letter(tavern: Tavern) -> char {
    tavern_menu_letters(tavern).secondary
}

pub const fn tavern_provisions_menu_letter(tavern: Tavern) -> Option<char> {
    tavern_menu_letters(tavern).provisions
}

pub const fn tavern_lore_menu_letter(tavern: Tavern) -> char {
    tavern_menu_letters(tavern).lore
}

pub const fn arms_buy_quote_record_id_for_item(item_id: u8) -> Option<usize> {
    let item = item_id as usize;
    match item {
        0..=7 => Some(8 + item),
        9..=14 => Some(16 + (item - 9)),
        16..=34 => Some(22 + (item - 16)),
        36..=38 => Some(41 + (item - 36)),
        42..=46 => Some(44 + (item - 42)),
        _ => None,
    }
}

pub const fn arms_buy_confirmation_prompt_for_roll(roll: u8) -> &'static str {
    match roll & 0x03 {
        0 => "Wouldst thou buy one?",
        1 => "Wilt thou take it?",
        2 => "Wish ye it?",
        _ => "May I get one for thee?",
    }
}

pub const fn arms_buy_confirmation_prompt(item_id: u8) -> &'static str {
    arms_buy_confirmation_prompt_for_roll(item_id)
}

/// `shops.md §8.1` (draw table) / `§8.A` resident-literal table: the
/// four-entry arms no-credit bark pool, published verbatim. One line is
/// chosen uniformly on draws `0..3` and the render site wraps it in the
/// shopkeeper-attribution tail `yells <shopkeeper>.`
///
/// *Corrected:* this pool previously held four invented polite refusals
/// ("I cannot extend thee credit." and friends). The sibling pools in this
/// module were already verbatim; this row was an isolated invention.
pub const fn arms_no_credit_bark_for_roll(roll: u8) -> &'static str {
    match roll & 0x03 {
        0 => "Can't pay?! Out with ye, orc-face!",
        1 => "What be ye trying to pull? OUT!",
        2 => "OUT, SLIME!",
        _ => "BEAT IT!",
    }
}

/// `shops.md §8.1` / `§8.A`: attribution tail wrapped around a drawn
/// arms no-credit bark. The shopkeeper name is substituted for
/// `<shopkeeper>`.
pub fn arms_no_credit_bark_with_attribution(bark: &str, shopkeeper: &str) -> String {
    format!("{bark}\nyells {shopkeeper}.")
}

pub const fn arms_no_credit_bark(item_id: u8) -> &'static str {
    arms_no_credit_bark_for_roll(item_id)
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
    quote_provision_purchase_at_unit_price(tavern, quantity, tavern_provision_unit_price(tavern))
}

pub const fn quote_provision_purchase_at_unit_price(
    tavern: Tavern,
    quantity: u16,
    unit_price: u16,
) -> Result<ProvisionPurchaseQuote, ProvisionPurchaseError> {
    if quantity == 0 {
        return Err(ProvisionPurchaseError::ZeroQuantity);
    }
    Ok(ProvisionPurchaseQuote {
        tavern,
        quantity,
        unit_price,
    })
}

pub fn apply_provision_purchase(
    gold: &mut u16,
    food: &mut u16,
    tavern: Tavern,
    quantity: u16,
) -> Result<ProvisionPurchaseOutcome, ProvisionPurchaseError> {
    apply_provision_purchase_at_unit_price(
        gold,
        food,
        tavern,
        quantity,
        tavern_provision_unit_price(tavern),
    )
}

pub fn apply_provision_purchase_at_unit_price(
    gold: &mut u16,
    food: &mut u16,
    tavern: Tavern,
    quantity: u16,
    unit_price: u16,
) -> Result<ProvisionPurchaseOutcome, ProvisionPurchaseError> {
    let quote = quote_provision_purchase_at_unit_price(tavern, quantity, unit_price)?;
    let gold_before = *gold;
    let food_before = *food;
    let mut purchased_quantity = 0u16;
    let completion = loop {
        if purchased_quantity == quantity {
            break ProvisionPurchaseCompletion::Completed;
        }
        if *gold < quote.unit_price {
            if purchased_quantity != 0 {
                break ProvisionPurchaseCompletion::GoldExhausted;
            }
            if *food >= TAVERN_PROVISION_CHARITY_THRESHOLD {
                return Err(ProvisionPurchaseError::NoNeed);
            }
            *food = food.saturating_add(1).min(SHOP_FOOD_STOCK_CAP);
            break ProvisionPurchaseCompletion::Charity;
        }

        *gold -= quote.unit_price;
        *food = food
            .saturating_add(TAVERN_PROVISION_PACK_SERVINGS)
            .min(SHOP_FOOD_STOCK_CAP);
        purchased_quantity += 1;
        if *food == SHOP_FOOD_STOCK_CAP {
            break ProvisionPurchaseCompletion::Completed;
        }
    };
    let total_price = gold_before - *gold;

    Ok(ProvisionPurchaseOutcome {
        quote,
        requested_quantity: quantity,
        purchased_quantity,
        total_price,
        gold_before,
        gold_after: *gold,
        food_before,
        food_after: *food,
        completion,
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
    base_room_rate: u16,
) -> Result<InnRestQuote, InnError> {
    if party_size == 0 {
        return Err(InnError::EmptyParty);
    }
    Ok(InnRestQuote {
        inn,
        party_size,
        base_room_rate,
        minimum_gold: inn_minimum_gold(inn),
        total_price: base_room_rate * party_size as u16,
    })
}

pub fn quote_inn_rest_for_speaker(
    inn: Inn,
    party_size: usize,
    speaker_intelligence: u8,
) -> Result<InnRestQuote, InnError> {
    if party_size == 0 {
        return Err(InnError::EmptyParty);
    }
    let base_room_rate = inn_base_room_rate(inn);
    let raw = base_room_rate * party_size as u16;
    Ok(InnRestQuote {
        inn,
        party_size,
        base_room_rate,
        minimum_gold: inn_minimum_gold(inn),
        total_price: shop_intelligence_adjusted_price(raw, speaker_intelligence),
    })
}

pub fn apply_inn_rest_payment(
    gold: &mut u16,
    inn: Inn,
    party_size: usize,
    base_room_rate: u16,
) -> Result<InnRestOutcome, InnError> {
    let quote = quote_inn_rest(inn, party_size, base_room_rate)?;
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

pub fn apply_inn_rest_total_payment(
    gold: &mut u16,
    inn: Inn,
    party_size: usize,
    total_price: u16,
) -> Result<InnRestOutcome, InnError> {
    if party_size == 0 {
        return Err(InnError::EmptyParty);
    }
    let minimum_gold = inn_minimum_gold(inn);
    if *gold < minimum_gold {
        return Err(InnError::BelowMinimumGold {
            available: *gold,
            minimum: minimum_gold,
        });
    }
    if *gold < total_price {
        return Err(InnError::InsufficientGold {
            available: *gold,
            required: total_price,
        });
    }

    let gold_before = *gold;
    *gold -= total_price;
    Ok(InnRestOutcome {
        quote: InnRestQuote {
            inn,
            party_size,
            base_room_rate: inn_base_room_rate(inn),
            minimum_gold,
            total_price,
        },
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
    base_lodging_charge: u16,
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
    let Some(registry_slot) = crate::free_inn_registry_slot(registry) else {
        return Err(InnError::RegistryFull);
    };
    if *gold < base_lodging_charge {
        return Err(InnError::InsufficientGold {
            available: *gold,
            required: base_lodging_charge,
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
        registry_slot,
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
    *gold -= base_lodging_charge;
    registry.push(guest);
    Ok(InnLeaveOutcome {
        scene_marker,
        party_index,
        registry_index: registry.len() - 1,
        deposit: base_lodging_charge,
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
    base_lodging_charge: u16,
) -> Result<InnPickupOutcome, InnError> {
    let Some(guest) = registry.get(registry_index).copied() else {
        return Err(InnError::InvalidGuestIndex {
            registry_len: registry.len(),
            requested: registry_index,
        });
    };
    let billable_stay_units = inn_billable_stay_units(guest.stay_counter);
    let bill = base_lodging_charge * u16::from(billable_stay_units);
    apply_inn_pickup_guest_with_bill(
        gold,
        registry,
        scene_marker,
        party,
        party_names,
        party_stay_counters,
        party_strengths,
        party_intelligence,
        party_experience,
        party_equipment,
        registry_index,
        bill,
    )
}

pub fn apply_inn_pickup_guest_with_bill(
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
    bill: u16,
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

pub fn sage_keyword_input_prefix(input: &str) -> &str {
    let trimmed = input.trim_start();
    let mut end = trimmed.len();
    for (count, (idx, _)) in trimmed.char_indices().enumerate() {
        if count == SAGE_KEYWORD_INPUT_LIMIT {
            end = idx;
            break;
        }
    }
    trimmed[..end].trim_end()
}

pub fn find_sage_topic(
    table: &SageRumourTable,
    input: &str,
) -> Result<SageRumourQuote, SageRumourError> {
    let capped = sage_keyword_input_prefix(input);
    if capped.is_empty() {
        return Err(SageRumourError::EmptyInput);
    }

    table
        .iter()
        .copied()
        .find(|entry| sage_topic_matches_input(entry.keyword, capped))
        .map(|entry| SageRumourQuote {
            entry,
            input_len: capped.len(),
        })
        .ok_or(SageRumourError::NoTopicMatch)
}

pub const fn sage_rumour_success_record_id_accepted(record_id: usize) -> bool {
    record_id >= SAGE_RUMOUR_SUCCESS_RECORD_FIRST && record_id <= SAGE_RUMOUR_SUCCESS_RECORD_LAST
}

pub fn sage_rumour_fallback_template(record_id: usize) -> &'static str {
    match record_id {
        85 => "Seek ye & in *!",
        86 => "Rumour says &, who lives in *, has knowledge.",
        87 => "It may be that &, of *, can help.",
        88 => "Mayhap & in * will aid the party.",
        _ => "Seek ye & in *!",
    }
}

pub fn render_sage_rumour(entry: SageRumourEntry, record_id: usize) -> String {
    sage_rumour_fallback_template(record_id)
        .replace('&', entry.subject)
        .replace('*', entry.destination)
}

pub fn apply_sage_rumour_lookup(
    table: &SageRumourTable,
    input: &str,
    record_id: usize,
) -> Result<SageRumourOutcome, SageRumourError> {
    let quote = find_sage_topic(table, input)?;
    Ok(SageRumourOutcome {
        quote,
        record_id,
        rendered: render_sage_rumour(quote.entry, record_id),
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

pub const fn quote_horse_purchase_for_speaker(
    stable: Stable,
    speaker_intelligence: u8,
) -> HorsePurchaseQuote {
    HorsePurchaseQuote {
        stable,
        price: shop_intelligence_adjusted_price(stable_horse_price(stable), speaker_intelligence),
    }
}

pub const fn horse_purchase_active_object(x: usize, y: usize, z: i8) -> ActiveObject {
    ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
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

/// `shops.md §8.7` delivery cell for one shipwright row.
///
/// These four pairs are **table data**, published verbatim beside the two
/// base prices in the same resident shop row. §8.7: "The delivery
/// coordinate is a per-shipwright value held beside the price rows in the
/// same resident shop table; it is **not** the town's exterior entrance or
/// exit cell, and it is not derived from the scene-to-exit mapping in
/// `systems/town-mode.md` or `systems/doors-and-z-transitions.md`."
///
/// *Corrected:* this function previously returned each hosting scene's
/// exterior entry/return coordinate, which is exactly the derivation §8.7
/// withdraws. Every row is near — but not equal to — the town's own
/// entrance cell, so the derived values looked plausible and were wrong by
/// up to nine tiles.
pub const fn shipwright_delivery_coordinate(shipwright: Shipwright) -> (usize, usize) {
    match shipwright {
        Shipwright::IslandShipwrights => (39, 221),
        Shipwright::TheCrowsNest => (151, 21),
        Shipwright::TheOakenOar => (79, 109),
        Shipwright::TheRustyBucket => (138, 159),
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
            if skiffs & 0x7f == 0x7f {
                None
            } else {
                Some(PendingVehicleAcquisition::Frigate {
                    x,
                    y,
                    skiffs: (skiffs & 0x7f) + 1,
                })
            },
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
                aux3: 0,
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
        let outcome = {
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
            )?
        };
        self.sync_pending_vehicle_purchase_state(outcome);
        Ok(outcome)
    }

    pub(crate) fn sync_pending_vehicle_purchase_state(
        &mut self,
        outcome: ShipwrightPurchaseOutcome,
    ) {
        self.pending_vehicle_save = match (outcome.pending_before, outcome.pending_after) {
            (_, Some(after)) => PendingVehicleSaveState::from_acquisition(after),
            (Some(before), None) => PendingVehicleSaveState::from_acquisition(before).clear_class(),
            (None, None) => self.pending_vehicle_save,
        };
    }

    pub fn buy_horse(
        &mut self,
        stable: Stable,
        x: usize,
        y: usize,
    ) -> Result<HorsePurchaseOutcome, HorsePurchaseError> {
        let quote = quote_horse_purchase(stable);
        self.buy_horse_with_quote(quote, x, y)
    }

    pub fn buy_horse_for_price(
        &mut self,
        stable: Stable,
        price: u16,
        x: usize,
        y: usize,
    ) -> Result<HorsePurchaseOutcome, HorsePurchaseError> {
        self.buy_horse_with_quote(HorsePurchaseQuote { stable, price }, x, y)
    }

    fn buy_horse_with_quote(
        &mut self,
        quote: HorsePurchaseQuote,
        x: usize,
        y: usize,
    ) -> Result<HorsePurchaseOutcome, HorsePurchaseError> {
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
        self.mark_visibility_dirty();

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

    pub fn consult_sage_rumour(
        &mut self,
        table: &SageRumourTable,
        input: &str,
        record_id: usize,
    ) -> Result<SageRumourOutcome, SageRumourError> {
        apply_sage_rumour_lookup(table, input, record_id)
    }

    pub fn pay_inn_rest(
        &mut self,
        inn: Inn,
        base_room_rate: u16,
    ) -> Result<InnRestOutcome, InnError> {
        apply_inn_rest_payment(&mut self.gold, inn, self.party.len(), base_room_rate)
    }

    pub fn pay_inn_rest_total(
        &mut self,
        inn: Inn,
        total_price: u16,
    ) -> Result<InnRestOutcome, InnError> {
        apply_inn_rest_total_payment(&mut self.gold, inn, self.party.len(), total_price)
    }

    pub fn leave_inn_companion(
        &mut self,
        scene_marker: u8,
        party_index: usize,
        base_lodging_charge: u16,
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
            base_lodging_charge,
        )
    }

    pub fn pickup_inn_guest(
        &mut self,
        scene_marker: u8,
        registry_index: usize,
        base_lodging_charge: u16,
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
            base_lodging_charge,
        )
    }

    pub fn pickup_inn_guest_with_bill(
        &mut self,
        scene_marker: u8,
        registry_index: usize,
        bill: u16,
    ) -> Result<InnPickupOutcome, InnError> {
        apply_inn_pickup_guest_with_bill(
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
            bill,
        )
    }
}
