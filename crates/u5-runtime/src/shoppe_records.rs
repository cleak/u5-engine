//! Per-shop SHOPPE.DAT record selection helpers per
//! `formats/shoppe-dat.md` §3 and `systems/shops.md` §4.
//!
//! Each shop kind reads from a fixed band of records inside the
//! shared SHOPPE.DAT file. The bands tile contiguously starting at
//! record 0 (shared barks 0-7), then per-kind clusters. This module
//! exposes the per-kind first/last record ids and convenience pickers
//! that select one record from a band based on a hash, the time of
//! day, or a fixed offset.

use crate::shops::*;

/// Identifies which SHOPPE.DAT band a record id falls into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShoppeBand {
    SharedBark,
    ArmsDescription,
    ArmsSell,
    Tavern,
    Sage,
    HorseTrader,
    ShipBroker,
    Reagent,
    Guild,
    Healer,
    Innkeeper,
}

impl ShoppeBand {
    /// Inclusive (first, last) record-id pair for this band.
    pub const fn range(self) -> (usize, usize) {
        match self {
            Self::SharedBark => (
                SHOPPE_RECORDS_SHARED_BARKS_FIRST,
                SHOPPE_RECORDS_SHARED_BARKS_LAST,
            ),
            Self::ArmsDescription => (
                SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST,
                SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST,
            ),
            Self::ArmsSell => (
                SHOPPE_RECORDS_ARMS_SELL_FIRST,
                SHOPPE_RECORDS_ARMS_SELL_LAST,
            ),
            Self::Tavern => (SHOPPE_RECORDS_TAVERN_FIRST, SHOPPE_RECORDS_TAVERN_LAST),
            Self::Sage => (SHOPPE_RECORDS_SAGE_FIRST, SHOPPE_RECORDS_SAGE_LAST),
            Self::HorseTrader => (
                SHOPPE_RECORDS_HORSE_TRADER_FIRST,
                SHOPPE_RECORDS_HORSE_TRADER_LAST,
            ),
            Self::ShipBroker => (
                SHOPPE_RECORDS_SHIP_BROKER_FIRST,
                SHOPPE_RECORDS_SHIP_BROKER_LAST,
            ),
            Self::Reagent => (SHOPPE_RECORDS_REAGENT_FIRST, SHOPPE_RECORDS_REAGENT_LAST),
            Self::Guild => (SHOPPE_RECORDS_GUILD_FIRST, SHOPPE_RECORDS_GUILD_LAST),
            Self::Healer => (SHOPPE_RECORDS_HEALER_FIRST, SHOPPE_RECORDS_HEALER_LAST),
            Self::Innkeeper => (
                SHOPPE_RECORDS_INNKEEPER_FIRST,
                SHOPPE_RECORDS_INNKEEPER_LAST,
            ),
        }
    }

    /// Number of record slots in this band.
    pub const fn len(self) -> usize {
        let (first, last) = self.range();
        last - first + 1
    }

    /// Returns `true` when the record id lies within this band.
    pub const fn contains(self, id: usize) -> bool {
        let (first, last) = self.range();
        id >= first && id <= last
    }

    /// Classify a record id into its band, picking the most specific
    /// match when bands overlap (sage records sit inside the tavern
    /// band).
    pub fn classify(id: usize) -> Option<Self> {
        let candidates = [
            Self::SharedBark,
            Self::ArmsDescription,
            Self::ArmsSell,
            Self::Sage, // before Tavern so the overlap resolves to Sage
            Self::Tavern,
            Self::HorseTrader,
            Self::ShipBroker,
            Self::Reagent,
            Self::Guild,
            Self::Healer,
            Self::Innkeeper,
        ];
        candidates.into_iter().find(|band| band.contains(id))
    }

    /// Select one record-id from the band by hashing the supplied
    /// seed against the band's length.
    pub const fn pick_by_seed(self, seed: u64) -> usize {
        let (first, _) = self.range();
        first + (seed % (self.len() as u64)) as usize
    }
}

/// Pick a SHOPPE.DAT record id for the shop kind identified by the
/// Talk-resolved trigger byte (`shops.md §2`). Returns the first
/// record id in the matching band, suitable as a default greeting /
/// bark. Returns `None` for non-shop dialog ids.
pub const fn shoppe_default_record_for_dialog_id(dialog_id: u8) -> Option<usize> {
    let band = match dialog_id {
        0x81 => ShoppeBand::ArmsDescription,
        0x82 => ShoppeBand::Tavern,
        0x83 => ShoppeBand::HorseTrader,
        0x84 => ShoppeBand::ShipBroker,
        0x85 => ShoppeBand::Reagent,
        0x86 => ShoppeBand::Guild,
        0x87 => ShoppeBand::Healer,
        0x88 => ShoppeBand::Innkeeper,
        _ => return None,
    };
    Some(band.range().0)
}

/// Pick a shared-bark record id by hashing the supplied seed.
pub const fn shared_bark_record_by_seed(seed: u64) -> usize {
    ShoppeBand::SharedBark.pick_by_seed(seed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedShopBarkKind {
    Preamble,
    InitialGreeting,
    Farewell,
}

const SHARED_SHOP_BARK_UNUSED: usize = usize::MAX;

const SHARED_SHOP_PREAMBLE_FIRST: [usize; 8] =
    [SHARED_SHOP_BARK_UNUSED, 57, 92, 105, 127, 148, 165, 174];
const SHARED_SHOP_INITIAL_FIRST: [usize; 8] = [0, 61, 96, 109, 131, 152, 169, 178];
const SHARED_SHOP_FAREWELL_FIRST: [usize; 8] = [4, 65, 100, 113, 135, 156, 169, 182];

pub const fn shop_trigger_row(dialog_id: u8) -> Option<usize> {
    if dialog_id >= 0x81 && dialog_id <= 0x88 {
        Some((dialog_id - 0x81) as usize)
    } else {
        None
    }
}

pub const fn shared_shop_bark_record(
    dialog_id: u8,
    kind: SharedShopBarkKind,
    ordinal: u8,
) -> Option<usize> {
    let Some(row) = shop_trigger_row(dialog_id) else {
        return None;
    };
    let first = match kind {
        SharedShopBarkKind::Preamble => SHARED_SHOP_PREAMBLE_FIRST[row],
        SharedShopBarkKind::InitialGreeting => SHARED_SHOP_INITIAL_FIRST[row],
        SharedShopBarkKind::Farewell => SHARED_SHOP_FAREWELL_FIRST[row],
    };
    if first == SHARED_SHOP_BARK_UNUSED || ordinal > 3 {
        None
    } else {
        Some(first + ordinal as usize)
    }
}

pub const fn talk_entry_uses_shared_preamble(dialog_id: u8) -> bool {
    matches!(dialog_id, 0x83 | 0x85 | 0x86 | 0x87)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_ranges_match_published_constants() {
        assert_eq!(ShoppeBand::SharedBark.range(), (0, 7));
        assert!(ShoppeBand::SharedBark.contains(0));
        assert!(ShoppeBand::SharedBark.contains(7));
        assert!(!ShoppeBand::SharedBark.contains(8));
    }

    #[test]
    fn band_lengths_are_inclusive_widths() {
        assert_eq!(ShoppeBand::SharedBark.len(), 8);
        assert_eq!(
            ShoppeBand::ArmsDescription.len(),
            SHOPPE_RECORDS_ARMS_DESCRIPTIONS_LAST - SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST + 1
        );
    }

    #[test]
    fn classify_returns_the_matching_band_for_each_id() {
        assert_eq!(ShoppeBand::classify(0), Some(ShoppeBand::SharedBark));
        assert_eq!(
            ShoppeBand::classify(SHOPPE_RECORDS_HEALER_FIRST),
            Some(ShoppeBand::Healer)
        );
        assert_eq!(
            ShoppeBand::classify(SHOPPE_RECORDS_INNKEEPER_LAST),
            Some(ShoppeBand::Innkeeper)
        );
    }

    #[test]
    fn classify_resolves_sage_tavern_overlap_to_sage() {
        // Records 84-91 sit inside both Tavern (57-88) and Sage
        // (84-91). The classifier prefers the narrower Sage band.
        assert_eq!(ShoppeBand::classify(84), Some(ShoppeBand::Sage));
        assert_eq!(ShoppeBand::classify(91), Some(ShoppeBand::Sage));
    }

    #[test]
    fn pick_by_seed_stays_within_band() {
        for seed in 0u64..32 {
            let id = ShoppeBand::Tavern.pick_by_seed(seed);
            assert!(ShoppeBand::Tavern.contains(id), "seed {seed} → id {id}");
        }
    }

    #[test]
    fn dialog_id_dispatch_maps_to_correct_first_record() {
        assert_eq!(
            shoppe_default_record_for_dialog_id(0x81),
            Some(SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST)
        );
        assert_eq!(
            shoppe_default_record_for_dialog_id(0x82),
            Some(SHOPPE_RECORDS_TAVERN_FIRST)
        );
        assert_eq!(
            shoppe_default_record_for_dialog_id(0x88),
            Some(SHOPPE_RECORDS_INNKEEPER_FIRST)
        );
    }

    #[test]
    fn dialog_id_dispatch_rejects_non_shop_ids() {
        assert_eq!(shoppe_default_record_for_dialog_id(0x00), None);
        assert_eq!(shoppe_default_record_for_dialog_id(0x80), None);
        assert_eq!(shoppe_default_record_for_dialog_id(0x89), None);
    }

    #[test]
    fn shared_bark_record_by_seed_returns_within_shared_band() {
        for seed in 0u64..32 {
            let id = shared_bark_record_by_seed(seed);
            assert!(ShoppeBand::SharedBark.contains(id));
        }
    }

    #[test]
    fn shared_shop_bark_rows_match_published_table() {
        assert_eq!(
            shared_shop_bark_record(0x81, SharedShopBarkKind::Preamble, 0),
            None
        );
        assert_eq!(
            shared_shop_bark_record(0x81, SharedShopBarkKind::InitialGreeting, 0),
            Some(0)
        );
        assert_eq!(
            shared_shop_bark_record(0x81, SharedShopBarkKind::Farewell, 3),
            Some(7)
        );
        assert_eq!(
            shared_shop_bark_record(0x82, SharedShopBarkKind::Preamble, 3),
            Some(60)
        );
        assert_eq!(
            shared_shop_bark_record(0x83, SharedShopBarkKind::InitialGreeting, 2),
            Some(98)
        );
        assert_eq!(
            shared_shop_bark_record(0x84, SharedShopBarkKind::Farewell, 1),
            Some(114)
        );
        assert_eq!(
            shared_shop_bark_record(0x85, SharedShopBarkKind::Preamble, 0),
            Some(127)
        );
        assert_eq!(
            shared_shop_bark_record(0x86, SharedShopBarkKind::InitialGreeting, 3),
            Some(155)
        );
        assert_eq!(
            shared_shop_bark_record(0x87, SharedShopBarkKind::Farewell, 0),
            Some(169)
        );
        assert_eq!(
            shared_shop_bark_record(0x88, SharedShopBarkKind::Farewell, 3),
            Some(185)
        );
        assert_eq!(
            shared_shop_bark_record(0x88, SharedShopBarkKind::Farewell, 4),
            None
        );
    }

    #[test]
    fn talk_entry_preamble_is_limited_to_published_shop_flows() {
        assert!(!talk_entry_uses_shared_preamble(0x81));
        assert!(!talk_entry_uses_shared_preamble(0x82));
        assert!(talk_entry_uses_shared_preamble(0x83));
        assert!(!talk_entry_uses_shared_preamble(0x84));
        assert!(talk_entry_uses_shared_preamble(0x85));
        assert!(talk_entry_uses_shared_preamble(0x86));
        assert!(talk_entry_uses_shared_preamble(0x87));
        assert!(!talk_entry_uses_shared_preamble(0x88));
    }

    #[test]
    fn every_shop_band_is_classifiable_by_its_first_id() {
        for band in [
            ShoppeBand::ArmsDescription,
            ShoppeBand::ArmsSell,
            ShoppeBand::Tavern,
            ShoppeBand::HorseTrader,
            ShoppeBand::ShipBroker,
            ShoppeBand::Reagent,
            ShoppeBand::Guild,
            ShoppeBand::Healer,
            ShoppeBand::Innkeeper,
        ] {
            let id = band.range().0;
            assert!(band.contains(id));
        }
    }
}
